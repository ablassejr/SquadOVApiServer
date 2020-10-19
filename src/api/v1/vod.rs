mod create;
mod delete;
mod find;
mod get;
mod filesystem_vod_manager;

use crate::common;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize,Deserialize};

pub enum VodManagerType {
    FileSystem,
    GCS
}

pub fn get_current_vod_manager_type() -> VodManagerType {
    let root = std::env::var("SQUADOV_VOD_ROOT").unwrap();
    if root.starts_with("gs://") {
        VodManagerType::GCS
    } else {
        VodManagerType::FileSystem
    }
}

#[derive(Serialize,Deserialize)]
pub struct VodAssociation {
    #[serde(rename = "matchUuid")]
    match_uuid: Option<Uuid>,
    #[serde(rename = "userUuid")]
    user_uuid: Option<Uuid>,
    #[serde(rename = "videoUuid")]
    video_uuid: Uuid,
    #[serde(rename = "startTime")]
    start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endTime")]
    end_time: Option<DateTime<Utc>>
}

#[derive(Serialize)]
pub struct VodVideoQualityOption {
    option: String,
    bandwidth: i64,
    width: i32,
    height: i32,
    codec: String,
}

#[async_trait]
pub trait VodManager {
    async fn get_vod_video_quality_options(&self, video_uuid: &Uuid) -> Result<Vec<VodVideoQualityOption>, common::SquadOvError>;
}

pub use create::*;
pub use delete::*;
pub use find::*;
pub use get::*;
pub use filesystem_vod_manager::*;