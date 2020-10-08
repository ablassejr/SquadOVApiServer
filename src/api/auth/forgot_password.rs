use actix_web::{HttpResponse, web};
use serde::Deserialize;
use crate::logged_error;
use crate::api;
use crate::api::fusionauth;
use crate::common;

#[derive(Deserialize)]
pub struct ForgotPasswordInputs {
    #[serde(rename = "loginId")]
    login_id: String,
}

#[derive(Deserialize)]
pub struct ChangePasswordInputs {
    #[serde(rename = "changePasswordId")]
    change_password_id: String,
    password: String,
}

async fn forgot_pw(fa: &fusionauth::FusionAuthClient, login_id: &str) -> Result<(), common::SquadOvError> {
    match fa.start_forgot_password_workflow(login_id).await {
        Ok(_) => Ok(()),
        Err(err) => match err {
            // Handle this case especially since we don't want to present to the caller data about whether or not that particular
            // user exists.
            fusionauth::FusionAuthResendVerificationEmailError::DoesNotExist => Ok(()),
            _ => Err(common::SquadOvError::InternalError(format!("Start Forgot Password Workflow: {}", err)))
        }
    }
}

async fn change_pw(fa: &fusionauth::FusionAuthClient, change_id: &str, password: &str) -> Result<(), common::SquadOvError> {
    match fa.change_user_password(change_id, password).await {
        Ok(_) => Ok(()),
        Err(err) => Err(common::SquadOvError::InternalError(format!("Change Password: {}", err))),
    }
}

/// Starts the password reset flow. Note that no error is given if
/// the specified user doesn't exist.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (no email sent).
pub async fn forgot_pw_handler(data : web::Query<ForgotPasswordInputs>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    match forgot_pw(&app.clients.fusionauth, &data.login_id).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Changes the user password given that they started the forgot password flow.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (password was not  changed).
pub async fn forgot_pw_change_handler(app : web::Data<api::ApiApplication>, data : web::Json<ChangePasswordInputs>) -> Result<HttpResponse, common::SquadOvError> {
    match change_pw(&app.clients.fusionauth, &data.change_password_id, &data.password).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}