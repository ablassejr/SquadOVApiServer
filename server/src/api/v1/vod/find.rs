use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;
use squadov_common::vod::VodAssociation;

#[derive(Deserialize)]
pub struct VodMatchFindFromMatchUserId {
    match_uuid: Uuid,
    user_id: i64
}

impl api::ApiApplication {
    pub async fn find_vods_without_fastify(&self) -> Result<Vec<Uuid>, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT vm.video_uuid
            FROM squadov.vod_metadata AS vm
            INNER JOIN squadov.vods AS v
                ON v.video_uuid = vm.video_uuid
            WHERE has_fastify = false
                AND end_time BETWEEN NOW() - INTERVAL '1 months' AND NOW()
            "
        )
            .fetch_all(&*self.pool)
            .await?)
    }

    pub async fn find_vods_without_preview(&self) -> Result<Vec<Uuid>, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT vm.video_uuid
            FROM squadov.vod_metadata AS vm
            INNER JOIN squadov.vods AS v
                ON v.video_uuid = vm.video_uuid
            WHERE has_preview = false
                AND end_time BETWEEN NOW() - INTERVAL '1 months' AND NOW()
            "
        )
            .fetch_all(&*self.pool)
            .await?)
    }

    pub async fn find_vod_from_match_user_id(&self, match_uuid: Uuid, user_id: i64) -> Result<Option<VodAssociation>, SquadOvError> {
        Ok(sqlx::query_as!(
            VodAssociation,
            "
            SELECT v.*
            FROM squadov.vods AS v
            INNER JOIN squadov.users AS u
                ON u.uuid = v.user_uuid
            WHERE v.match_uuid = $1
                AND u.id = $2
                AND v.is_clip = FALSE
            ",
            match_uuid,
            user_id,
        )
            .fetch_optional(&*self.pool)
            .await?)
    }

    pub async fn find_accessible_vods_in_match_for_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<VodAssociation>, SquadOvError> {
        Ok(sqlx::query_as!(
            VodAssociation,
            "
            SELECT DISTINCT v.*
            FROM squadov.vods AS v
            INNER JOIN squadov.users AS u
                ON u.uuid = v.user_uuid
            LEFT JOIN squadov.view_share_connections_access_users AS vau
                ON vau.video_uuid = v.video_uuid
                    AND vau.match_uuid = $1
                    AND vau.user_id = $2
            WHERE v.match_uuid = $1 
                AND (u.id = $2 OR vau.video_uuid IS NOT NULL)
                AND v.is_clip = FALSE
                AND (v.is_local = FALSE OR u.id = $2)
            ",
            match_uuid,
            user_id,
        )
            .fetch_all(&*self.pool)
            .await?)
    }

    pub async fn find_vod_associations(&self, video_uuid: &[Uuid]) -> Result<HashMap<Uuid, VodAssociation>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                VodAssociation,
                "
                SELECT v.*
                FROM squadov.vods AS v
                WHERE v.video_uuid = ANY($1)
                ",
                video_uuid,
            )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| {
                (x.video_uuid.clone(), x)
            })
            .collect()
        )
    }

    pub async fn get_user_full_match_vod_count(&self, user_id: i64) -> Result<i64, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT COUNT(v.video_uuid) AS "count!"
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE u.id = $1
                    AND v.is_clip = FALSE
                    AND v.end_time IS NOT NULL
                "#,
                user_id
            )
                .fetch_one(&*self.pool)
                .await?
                .count
        )
    }
}

pub async fn find_vod_from_match_user_id_handler(data : web::Path<VodMatchFindFromMatchUserId>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let assoc = app.find_vod_from_match_user_id(data.match_uuid, data.user_id).await?;
    match assoc {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Err(SquadOvError::NotFound),
    }
}