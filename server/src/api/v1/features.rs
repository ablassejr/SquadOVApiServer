use serde::Serialize;
use squadov_common::{SquadOvError};
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    user_id: i64,
    max_record_pixel_y: i32,
    max_record_fps: i32,
    allow_record_upload: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            user_id: -1,
            max_record_pixel_y: 1080,
            max_record_fps: 60,
            allow_record_upload: true,
        }
    }
}

pub async fn get_user_feature_flags_handler(app : web::Data<Arc<api::ApiApplication>>, data : web::Path<super::UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let flags = sqlx::query_as!(
        FeatureFlags,
        "
        SELECT *
        FROM squadov.user_feature_flags
        WHERE user_id = $1
        ",
        data.user_id,
    )
        .fetch_optional(&*app.pool)
        .await?;

    if flags.is_some() {
        Ok(HttpResponse::Ok().json(&flags.unwrap()))
    } else {
        Ok(HttpResponse::Ok().json(FeatureFlags{
            user_id: data.user_id,
            ..FeatureFlags::default()
        }))
    }
}