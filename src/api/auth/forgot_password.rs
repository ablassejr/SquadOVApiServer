use actix_web::{HttpResponse};
use crate::logged_error;

async fn forgot_pw() -> Result<(), super::AuthError> {
    return Ok(())
}

/// Starts the password reset flow. Note that no error is given if
/// the specified user doesn't exist.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (no email sent).
pub async fn forgot_pw_handler() -> Result<HttpResponse, super::AuthError> {
    match forgot_pw().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Changes the user password given that they started the forgot password flow.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (password was not  changed).
pub async fn forgot_pw_change_handler() -> Result<HttpResponse, super::AuthError> {
    match forgot_pw().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}