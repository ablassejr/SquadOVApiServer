use actix_web::{HttpResponse, HttpRequest, web};
use crate::logged_error;
use crate::api;
use crate::api::fusionauth;

async fn logout(fa: &fusionauth::FusionAuthClient, refresh_token: &str) -> Result<(), super::AuthError> {
    match fa.logout(refresh_token).await {
        Ok(_) => Ok(()),
        Err(err) => Err(super::AuthError::System{
            message: format!("Failed to logout: {}", err)
        })
    }
}

/// Logouts the user with SquadOV and FusionAuth.
/// 
/// Possible Responses:
/// * 200 - Logout succeded.
/// * 500 - Logout failed.
pub async fn logout_handler(app : web::Data<api::ApiApplication>, req: HttpRequest) -> Result<HttpResponse, super::AuthError> {
    let session = match app.session.get_session_from_request(&req, &app.pool).await {
        Ok(x) => match x {
            Some(y) => y,
            // They weren't logged in to begin with so it's OK to just tell them
            // they logged out successfully.
            None => return Ok(HttpResponse::Ok().finish()),
        },
        Err(err) => return logged_error!(super::AuthError::System{
            message: format!("Logout {}", err)
        })
    };

    match logout(&app.clients.fusionauth, &session.refresh_token).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}