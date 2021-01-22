use serde::Serialize;
use squadov_common::{SquadOvError};
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    enable_lol: bool,
    enable_tft: bool
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_lol: false,
            enable_tft: false,
        }
    }
}

pub async fn get_user_feature_flags_handler(app : web::Data<Arc<api::ApiApplication>>, data : web::Path<super::UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let flags = sqlx::query_as!(
        FeatureFlags,
        "
        SELECT
            enable_lol,
            enable_tft
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
        Ok(HttpResponse::Ok().json(FeatureFlags::default()))
    }
}