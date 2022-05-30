use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize};
use sqlx::{Transaction, Postgres};
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
    r#ref: Option<String>
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RegisterParams {
    invite_uuid: Option<Uuid>,
    squad_id: Option<i64>,
    sig: Option<String>
}

async fn register(fa: &fusionauth::FusionAuthClient, data: RegisterData) -> Result<String, SquadOvError> {
    let output = fa.register(fa.build_register_input(
        data.username,
        data.email,
        data.password,
    )).await?;

    Ok(output.user.email)
}

impl api::ApiApplication {
    async fn associate_user_to_referral_code(&self, tx: &mut Transaction<'_, Postgres>, email: &str, referral_code: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.user_referral_code_usage (
                email,
                code_id,
                tm
            )
            SELECT $1, id, NOW()
            FROM squadov.referral_codes
            WHERE code = $2
            ",
            email,
            referral_code
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn get_referral_code_associated_to_user(&self, user_id: i64) -> Result<Option<String>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT rc.code
                FROM squadov.user_referral_code_usage AS ucu
                INNER JOIN squadov.referral_codes AS rc
                    ON rc.id = ucu.code_id
                INNER JOIN squadov.users AS u
                    ON u.email = ucu.email
                WHERE u.id = $1
                ",
                user_id
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.code
                })
        )
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

    let mut data = data.into_inner();
    data.email = data.email.to_lowercase().trim().to_string();

    let referral = data.r#ref.clone();
    let email = register(&app.clients.fusionauth, data).await?;

    let mut tx = app.pool.begin().await?;
    if let Some(referral_code) = &referral {
        app.associate_user_to_referral_code(&mut tx, &email, referral_code).await?;
    }

    if aux.invite_uuid.is_some() && aux.squad_id.is_some() && aux.sig.is_some() {
        let squad_id = aux.squad_id.unwrap();
        let invite_uuid = aux.invite_uuid.unwrap();
        let test_sig = aux.sig.as_ref().unwrap().clone();

        let sig = app.generate_invite_hmac_signature(squad_id, &invite_uuid)?;
        if sig != test_sig {
            return Err(SquadOvError::Unauthorized);
        }
        
        // Reassociate this invite with whatever email the user just used to register (this allows
        // an invite to be used for another email if the inviter didn't use the user's desired email address).
        app.reassociate_invite_email(&mut tx, &invite_uuid, &email).await?;

        // Flag this invite as needing to be applied (whenever the user next logs in).
        app.set_invite_pending(&mut tx, &invite_uuid, true).await?;

        // Accept invite (consumes the invite so it can't be re-used)
        app.accept_reject_invite(&mut tx, squad_id, &invite_uuid, true).await?;
    }

    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}