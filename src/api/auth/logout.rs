use actix_web::{HttpResponse, HttpRequest, web};
use crate::logged_error;
use crate::api;
use crate::common;

/// Logouts the user with SquadOV and FusionAuth.
/// 
/// Possible Responses:
/// * 200 - Logout succeded.
/// * 500 - Logout failed. An error occcurred but the logout *may have* succeeded.
pub async fn logout_handler(app : web::Data<api::ApiApplication>, req: HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    // Use the session.get_session_from_request instead of app.refresh_and_obtain_valid_session_from_request
    // because it'd be a waste to refresh a session here.
    let session = match app.session.get_session_from_request(&req, &app.pool).await {
        Ok(x) => match x {
            Some(y) => y,
            // They weren't logged in to begin with so it's OK to just tell them
            // they logged out successfully.
            None => return Ok(HttpResponse::Ok().finish()),
        },
        Err(err) => return logged_error!(common::SquadOvError::InternalError(format!("Logout Find Session {}", err)))
    };

    match app.logout(&session).await {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(err) => logged_error!(err),
    }
}