use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize};
use crate::api;
use crate::api::fusionauth;
use crate::logged_error;

#[derive(Deserialize)]
pub struct RegisterData {
    username: String,
    email: String,
    password: String,
}

async fn register(fa: &fusionauth::FusionAuthClient, data: RegisterData) -> Result<(), super::AuthError> {
    let res = fa.register(fa.build_register_input(
        data.username,
        data.email,
        data.password,
    )).await;

    match res {
        Ok(_) => Ok(()),
        Err(err) => match err {
            fusionauth::FusionAuthRegisterError::InvalidRequest(x) => Err(super::AuthError::System{ message: x }),
            fusionauth::FusionAuthRegisterError::ServerAuth => Err(super::AuthError::System{ message: String::from("Server Auth.")}),
            fusionauth::FusionAuthRegisterError::InternalError => Err(super::AuthError::System{ message: String::from("Internal FA Error.")}),
            fusionauth::FusionAuthRegisterError::Search(x) => Err(super::AuthError::System{ message: format!("Search: {}", x)}),
            fusionauth::FusionAuthRegisterError::Generic(x) => Err(super::AuthError::System{ message: format!("Generic: {}", x)}),
        }
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
pub async fn register_handler(data : web::Json<RegisterData>, app : web::Data<api::ApiApplication>, req : HttpRequest) -> Result<HttpResponse, super::AuthError> {
    if app.session.is_logged_in(&req, &app.pool).await? {
        return logged_error!(super::AuthError::BadRequest);
    }

    match register(&app.clients.fusionauth, data.into_inner()).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}