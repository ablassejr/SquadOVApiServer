use squadov_common;
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;

pub async fn get_user_profile_handler(data : web::Path<super::UserResourcePath>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    match app.users.get_stored_user_from_id(data.user_id, &app.pool).await {
        Ok(x) => match x {
            Some(x) => Ok(HttpResponse::Ok().json(&x)),
            None => Err(squadov_common::SquadOvError::NotFound),
        },
        Err(err) => Err(squadov_common::SquadOvError::InternalError(format!("Get User Profile Handler {}", err))),
    }
}

pub async fn get_current_user_profile_handler(app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    let user = app.users.get_stored_user_from_id(session.user.id, &app.pool).await?;
    return Ok(HttpResponse::Ok().json(&user));
}