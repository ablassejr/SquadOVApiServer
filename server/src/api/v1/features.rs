use serde::Serialize;
use squadov_common::{
    SquadOvError,
    rabbitmq::RABBITMQ_DEFAULT_PRIORITY,
};
use sqlx::{Executor, Postgres};
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlags {
    pub user_id: i64,
    pub max_record_pixel_y: i32,
    pub max_record_fps: i32,
    pub allow_record_upload: bool,
    pub allow_wow_combat_log_upload: bool,
    pub enable_user_profiles: bool,
    pub disable_sentry: bool,
    pub max_bitrate_kbps: i32,
    pub can_instant_clip: bool,
    pub disable_es_search: bool,
    pub mandatory_watermark: bool,
    pub watermark_min_size: f64,
    pub vod_priority: i16,
    pub early_access: bool,
    pub vod_retention: Option<i64>,
    pub max_squad_size: Option<i64>,
    pub max_clip_seconds: i64,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            user_id: -1,
            max_record_pixel_y: 720,
            max_record_fps: 60,
            allow_record_upload: true,
            allow_wow_combat_log_upload: true,
            enable_user_profiles: true,
            disable_sentry: false,
            max_bitrate_kbps: 6000,
            can_instant_clip: true,
            disable_es_search: false,
            mandatory_watermark: true,
            watermark_min_size: 0.01,
            vod_priority: RABBITMQ_DEFAULT_PRIORITY as i16,
            early_access: false,
            vod_retention: Some(chrono::Duration::days(7).num_seconds()),
            max_squad_size: Some(20),
            max_clip_seconds: 120,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalFlags {
    disable_registration: bool,
}

impl Default for GlobalFlags {
    fn default() -> Self {
        Self {
            disable_registration: false,
        }
    }
}

pub async fn get_feature_flags<'a, T>(ex: T, user_id: i64) -> Result<FeatureFlags, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            FeatureFlags,
            "
            SELECT *
            FROM squadov.user_feature_flags
            WHERE user_id = $1
            ",
            user_id,
        )
            .fetch_optional(ex)
            .await?
            .unwrap_or(FeatureFlags{
                user_id,
                ..FeatureFlags::default()
            })
    )
}

pub async fn update_feature_flags<'a, T>(ex: T, user_id: i64, flags: FeatureFlags) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.user_feature_flags
        SET max_record_pixel_y = $2,
            max_record_fps = $3,
            allow_record_upload = $4,
            allow_wow_combat_log_upload = $5,
            enable_user_profiles = $6,
            disable_sentry = $7,
            max_bitrate_kbps = $8,
            can_instant_clip = $9,
            disable_es_search = $10,
            mandatory_watermark = $11,
            watermark_min_size = $12,
            vod_priority = $13,
            early_access = $14,
            vod_retention = $15,
            max_squad_size = $16
        WHERE user_id = $1
        ",
        user_id,
        flags.max_record_pixel_y,
        flags.max_record_fps,    
        flags.allow_record_upload,
        flags.allow_wow_combat_log_upload,
        flags.enable_user_profiles,
        flags.disable_sentry,
        flags.max_bitrate_kbps,
        flags.can_instant_clip,
        flags.disable_es_search,
        flags.mandatory_watermark,
        flags.watermark_min_size,
        flags.vod_priority,
        flags.early_access,
        flags.vod_retention,
        flags.max_squad_size,
    )
        .execute(ex)
        .await?;
    Ok(())
}

impl api::ApiApplication {
    pub async fn get_global_app_flags(&self) -> Result<GlobalFlags, SquadOvError> {
        let kvp_flags = sqlx::query!("
            SELECT *
            FROM squadov.global_app_flags
        ")
            .fetch_all(&*self.pool)
            .await?;

        let mut flags = GlobalFlags::default();
        for kvp in kvp_flags {
            match kvp.fkey.to_lowercase().as_str() {
                "disable_registration" => flags.disable_registration = kvp.fvalue.parse::<bool>()?,
                _ => (),
            };
        }
        Ok(flags)
    }
}

pub async fn get_user_feature_flags_handler(app : web::Data<Arc<api::ApiApplication>>, data : web::Path<super::UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(&get_feature_flags(&*app.pool, data.user_id).await?))
}

pub async fn get_global_app_flags_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(app.get_global_app_flags().await?))
}