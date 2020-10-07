use actix_web::{HttpResponse, HttpRequest};
use crate::logged_error;

async fn logout() -> Result<(), super::AuthError> {
    return Ok(())
}

/// Logouts the user with SquadOV and FusionAuth.
/// 
/// Possible Responses:
/// * 200 - Logout succeded.
/// * 500 - Logout failed.
pub async fn logout_handler(req: HttpRequest) -> Result<HttpResponse, super::AuthError> {
    if !super::is_logged_in(&req) {
        // They weren't logged in to begin with so it's OK to just tell them
        // they logged out successfully.
        return Ok(HttpResponse::Ok().finish());
    }

    match logout().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}
