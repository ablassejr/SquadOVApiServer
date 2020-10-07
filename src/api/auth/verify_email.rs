use actix_web::{HttpResponse, web};
use serde::{Deserialize};
use crate::api;
use crate::api::fusionauth;
use crate::logged_error;

#[derive(Deserialize)]
pub struct VerifyEmailData {
    #[serde(rename = "verificationId")]
    verification_id: String
}

async fn verify_email(fa: &fusionauth::FusionAuthClient, verification_id: &str) -> Result<(), super::AuthError> {
    match fa.verify_email(verification_id).await {
        Ok(_) => Ok(()),
        Err(err) => Err(super::AuthError::System{
            message: format!("Failed to verify email: {}", err)
        })
    }
}

async fn resend_verify_email(fa: &fusionauth::FusionAuthClient, email: &str) -> Result<(), super::AuthError> {
    return Ok(())
}

/// Verifies the user's email.
/// 
/// Possible Responses:
/// * 200 - Email verification succeded.
/// * 500 - Email verification failed.
pub async fn verify_email_handler(data : web::Json<VerifyEmailData>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, super::AuthError> {
    match verify_email(&app.clients.fusionauth, &data.verification_id).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Resends the user verification email.
/// 
/// Possible Responses:
/// * 200 - Email sent.
/// * 500 - Email was not sent.
pub async fn resend_verify_email_handler(data : web::Json<VerifyEmailData>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, super::AuthError> {
    match resend_verify_email(&app.clients.fusionauth, "").await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}