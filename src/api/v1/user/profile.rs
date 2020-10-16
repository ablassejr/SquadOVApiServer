use crate::common;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;
use crate::api;
use crate::api::auth::SquadOVSession;

#[derive(Deserialize)]
pub struct ProfileResource {
    user_id: i64,
}

pub async fn get_user_profile_handler(data : web::Path<ProfileResource>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    match app.users.get_stored_user_from_id(data.user_id, &app.pool).await {
        Ok(x) => match x {
            Some(x) => Ok(HttpResponse::Ok().json(&x)),
            None => Err(common::SquadOvError::NotFound),
        },
        Err(err) => Err(common::SquadOvError::InternalError(format!("Get User Profile Handler {}", err))),
    }
}


pub async fn get_current_user_profile_handler(app : web::Data<api::ApiApplication>, request : HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(common::SquadOvError::BadRequest)
    };

    let user = app.users.get_stored_user_from_id(session.user.id, &app.pool).await?;
    return Ok(HttpResponse::Ok().json(&user));
}