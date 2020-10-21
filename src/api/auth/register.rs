use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize};
use crate::api;
use crate::api::fusionauth;
use crate::common;
use crate::logged_error;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct RegisterData {
    username: String,
    email: String,
    password: String,
}

async fn register(fa: &fusionauth::FusionAuthClient, data: RegisterData) -> Result<(), common::SquadOvError> {
    let res = fa.register(fa.build_register_input(
        data.username,
        data.email,
        data.password,
    )).await;

    match res {
        Ok(_) => Ok(()),
        Err(err) => Err(common::SquadOvError::InternalError(format!("Register {}", err))),
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
pub async fn register_handler(data : web::Json<RegisterData>, app : web::Data<Arc<api::ApiApplication>>, req : HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    if app.is_logged_in(&req).await? {
        return logged_error!(common::SquadOvError::BadRequest);
    }

    match register(&app.clients.fusionauth, data.into_inner()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}