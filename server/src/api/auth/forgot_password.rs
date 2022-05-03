use actix_web::{HttpResponse, web, HttpRequest, HttpMessage};
use serde::{Serialize,Deserialize};
use crate::logged_error;
use crate::api;
use crate::api::fusionauth;
use crate::api::auth::SquadOVSession;
use squadov_common::SquadOvError;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ForgotPasswordInputs {
    #[serde(rename = "loginId")]
    login_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct ChangePasswordInputs {
    change_password_id: String,
    password: String,
    user_id: String,
    mfa_code: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ChangePasswordResponse {
    needs_mfa: bool
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct NewPasswordInputs {
    current_pw: String,
    new_pw: String,
    mfa_code: Option<String>,
}

async fn forgot_pw(fa: &fusionauth::FusionAuthClient, login_id: &str) -> Result<(), SquadOvError> {
    match fa.start_forgot_password_workflow(login_id).await {
        Ok(_) => Ok(()),
        Err(err) => match err {
            // Handle this case especially since we don't want to present to the caller data about whether or not that particular
            // user exists.
            fusionauth::FusionAuthResendVerificationEmailError::DoesNotExist => Ok(()),
            _ => Err(SquadOvError::InternalError(format!("Start Forgot Password Workflow: {}", err)))
        }
    }
}

async fn get_trust_token(fa: &fusionauth::FusionAuthClient, challenge: &str, user_id: Option<&str>, login_id: Option<&str>, mfa_code: &str) -> Result<String, SquadOvError> {
    let two_factor_id = fa.start_mfa(challenge, user_id, login_id).await?;
    Ok(fa.complete_mfa(mfa_code, &two_factor_id).await?)
}

async fn change_pw(fa: &fusionauth::FusionAuthClient, change_id: &str, password: &str, trust_challenge: Option<String>, trust_token: Option<String>) -> Result<(), SquadOvError> {
    match fa.change_user_password(change_id, password, trust_challenge, trust_token).await {
        Ok(_) => Ok(()),
        Err(err) => {
            match err {
                fusionauth::FusionAuthUserError::InvalidRequest(_j) => {
                    Err(SquadOvError::TwoFactor(String::new()))
                },
                _ => Err(SquadOvError::InternalError(format!("Change Password: {}", err))),
            }
        }
    }
}

/// Starts the password reset flow. Note that no error is given if
/// the specified user doesn't exist.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (no email sent).
pub async fn forgot_pw_handler(data : web::Query<ForgotPasswordInputs>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    match forgot_pw(&app.clients.fusionauth, &data.login_id).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(err) => logged_error!(err),
    }
}

/// Changes the user password given that they started the forgot password flow.
/// 
/// Possible Responses:
/// * 200 - Success.
/// * 500 - Internal error (password was not  changed).
pub async fn forgot_pw_change_handler(app : web::Data<Arc<api::ApiApplication>>, data : web::Json<ChangePasswordInputs>) -> Result<HttpResponse, SquadOvError> {
    let (trust_challenge, trust_token) = if let Some(mfa) = data.mfa_code.as_ref() {
        (Some(String::from("FORGOT_PW")), Some(get_trust_token(&app.clients.fusionauth, "FORGOT_PW", Some(&data.user_id), None, &mfa).await?))
    } else {
        (None, None)
    };

    match change_pw(&app.clients.fusionauth, &data.change_password_id, &data.password, trust_challenge, trust_token).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(err) => match err {
            SquadOvError::TwoFactor(_c) => Ok(HttpResponse::Ok().json(ChangePasswordResponse{
                needs_mfa: true,
            })),
            _ => logged_error!(err),
        }
    }
}

pub async fn change_pw_handler(app : web::Data<Arc<api::ApiApplication>>, data : web::Json<NewPasswordInputs>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let (trust_challenge, trust_token) = if let Some(mfa) = data.mfa_code.as_ref() {
        (Some(String::from("CHANGE_PW")), Some(get_trust_token(&app.clients.fusionauth, "CHANGE_PW", None, Some(&session.user.email), &mfa).await?))
    } else {
        (None, None)
    };

    match app.clients.fusionauth.change_user_password_with_id(&data.current_pw, &data.new_pw, &session.user.email, trust_challenge, trust_token).await {
        Ok(_) => Ok(HttpResponse::NoContent().finish()),
        Err(err) => {
            match err {
                SquadOvError::TwoFactor(_c) => Ok(HttpResponse::Ok().json(ChangePasswordResponse{
                    needs_mfa: true,
                })),
                _ => Err(err),
            }
        }
    }
}