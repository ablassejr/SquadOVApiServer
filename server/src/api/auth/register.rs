use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize};
use crate::api;
use crate::api::fusionauth;
use squadov_common::SquadOvError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct RegisterData {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RegisterParams {
    invite_uuid: Option<Uuid>,
    squad_id: Option<i64>,
    sig: Option<String>
}

async fn register(fa: &fusionauth::FusionAuthClient, data: RegisterData) -> Result<(), SquadOvError> {
    let res = fa.register(fa.build_register_input(
        data.username,
        data.email,
        data.password,
    )).await;

    match res {
        Ok(_) => Ok(()),
        Err(err) => Err(SquadOvError::InternalError(format!("Register {}", err))),
    }
}

/// Handles collecting the user data and passing it to FusionAuth for registration.
/// 
/// We expect only three parameters to be passed via the POST body: 
/// * Username
/// * Password
/// * Email
///
/// This function will not create a session. It is up to the application to redirect the user to
/// the login screen for them to login again.
/// 
/// Possible Responses:
/// * 200 - Registration succeeded.
/// * 400 - If a user is already logged in.
/// * 500 - Registration failed due to other reasons.
pub async fn register_handler(data : web::Json<RegisterData>, app : web::Data<Arc<api::ApiApplication>>, aux: web::Query<RegisterParams>, req : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    if app.is_logged_in(&req).await? {
        return Err(SquadOvError::BadRequest);
    }

    let email = data.email.clone();
    register(&app.clients.fusionauth, data.into_inner()).await?;
    if aux.invite_uuid.is_some() && aux.squad_id.is_some() && aux.sig.is_some() {
        let squad_id = aux.squad_id.unwrap();
        let invite_uuid = aux.invite_uuid.unwrap();
        let test_sig = aux.sig.as_ref().unwrap().clone();

        let sig = app.generate_invite_hmac_signature(squad_id, &invite_uuid)?;
        if sig != test_sig {
            return Err(SquadOvError::Unauthorized);
        }

        let mut tx = app.pool.begin().await?;
        // Reassociate this invite with whatever email the user just used to register (this allows
        // an invite to be used for another email if the inviter didn't use the user's desired email address).
        app.reassociate_invite_email(&mut tx, &invite_uuid, &email).await?;

        // Flag this invite as needing to be applied (whenever the user next logs in).
        app.set_invite_pending(&mut tx, &invite_uuid, true).await?;

        // Accept invite (consumes the invite so it can't be re-used)
        app.accept_reject_invite(&mut tx, squad_id, &invite_uuid, true).await?;

        tx.commit().await?;
    }

    Ok(HttpResponse::NoContent().finish())
}