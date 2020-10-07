use actix_web::{HttpResponse, HttpRequest, web};
use crate::logged_error;
use crate::api;

async fn logout() -> Result<(), super::AuthError> {
    return Ok(())
}

/// Logouts the user with SquadOV and FusionAuth.
/// 
/// Possible Responses:
/// * 200 - Logout succeded.
/// * 500 - Logout failed.
pub async fn logout_handler(app : web::Data<api::ApiApplication>, req: HttpRequest) -> Result<HttpResponse, super::AuthError> {
    if app.session.is_logged_in(&req, &app.pool).await? {
        // They weren't logged in to begin with so it's OK to just tell them
        // they logged out successfully.
        return Ok(HttpResponse::Ok().finish());
    }

    match logout().await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}
