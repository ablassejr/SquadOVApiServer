use squadov_common;
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;

impl api::ApiApplication {
    pub async fn get_user_local_encryption_password(&self, user_id: i64) -> Result<String, squadov_common::SquadOvError> {
        Ok(
            sqlx::query_scalar(
                "
                SELECT local_encryption_key
                FROM squadov.users
                WHERE id = $1
                "
            )
                .bind(user_id)
                .fetch_one(&*self.pool)
                .await?
        )
    }
}

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

pub async fn get_current_user_local_encryption_handler(app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    Ok(HttpResponse::Ok().json(&app.get_user_local_encryption_password(session.user.id).await?))
}