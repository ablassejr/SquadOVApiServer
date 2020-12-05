use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use uuid::Uuid;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct VodMatchFindFromMatchUserUuid {
    match_uuid: Uuid,
    user_uuid: Uuid
}

#[derive(Deserialize)]
pub struct VodMatchFindFromMatchUserId {
    match_uuid: Uuid,
    user_id: i64
}

impl api::ApiApplication {
    pub async fn find_vod_from_match_user_uuid(&self, match_uuid: Uuid, user_uuid: Uuid) -> Result<Option<super::VodAssociation>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
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
            .await?)
    }

    pub async fn find_vod_from_match_user_id(&self, match_uuid: Uuid, user_id: i64) -> Result<Option<super::VodAssociation>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            super::VodAssociation,
            "
            SELECT v.*
            FROM squadov.vods AS v
            INNER JOIN squadov.users AS u
                ON u.uuid = v.user_uuid
            WHERE v.match_uuid = $1
                AND u.id = $2
            ",
            match_uuid,
            user_id,
        )
            .fetch_optional(&*self.pool)
            .await?)
    }
}

pub async fn find_vod_from_match_user_uuid_handler(data : web::Path<VodMatchFindFromMatchUserUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let assoc = app.find_vod_from_match_user_uuid(data.match_uuid, data.user_uuid).await?;
    match assoc {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Err(squadov_common::SquadOvError::NotFound),
    }
}

pub async fn find_vod_from_match_user_id_handler(data : web::Path<VodMatchFindFromMatchUserId>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let assoc = app.find_vod_from_match_user_id(data.match_uuid, data.user_id).await?;
    match assoc {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Err(squadov_common::SquadOvError::NotFound),
    }
}