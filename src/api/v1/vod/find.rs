use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodMatchFindFromMatchUser {
    match_uuid: Uuid,
    user_uuid: Uuid
}

impl api::ApiApplication {
    pub async fn find_vod_from_match_user(&self, match_uuid: Uuid, user_uuid: Uuid) -> Result<Option<super::VodAssociation>, common::SquadOvError> {
        let vod = sqlx::query_as!(
            super::VodAssociation,
            "
            SELECT *
            FROM squadov.vods
            WHERE match_uuid = $1
                AND user_uuid = $2
            ",
            match_uuid,
            user_uuid,
        )
            .fetch_optional(&*self.pool)
            .await?;
        Ok(vod)
    }
}

pub async fn find_vod_from_match_user_handler(data : web::Path<VodMatchFindFromMatchUser>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let assoc = app.find_vod_from_match_user(data.match_uuid, data.user_uuid).await?;
    match assoc {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Err(common::SquadOvError::NotFound),
    }
}