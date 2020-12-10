mod create;
mod delete;
mod find;
mod get;
mod filesystem_vod_manager;
mod gcs_vod_manager;

use squadov_common;
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
    pub match_uuid: Option<Uuid>,
    #[serde(rename = "userUuid")]
    pub user_uuid: Option<Uuid>,
    #[serde(rename = "videoUuid")]
    pub video_uuid: Uuid,
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>
}

#[async_trait]
pub trait VodManager {
    async fn get_segment_upload_uri(&self, segment: &squadov_common::VodSegmentId) -> Result<String, squadov_common::SquadOvError>;
    async fn download_vod_to_path(&self, segment: &squadov_common::VodSegmentId, path: &std::path::Path) -> Result<(), squadov_common::SquadOvError>;
    async fn upload_vod_from_file(&self, segment: &squadov_common::VodSegmentId, path: &std::path::Path) -> Result<(), squadov_common::SquadOvError>;
    async fn get_segment_redirect_uri(&self, segment: &squadov_common::VodSegmentId) -> Result<String, squadov_common::SquadOvError>;
    async fn delete_vod(&self, segment: &squadov_common::VodSegmentId) -> Result<(), squadov_common::SquadOvError>;
}


pub use create::*;
pub use delete::*;
pub use find::*;
pub use get::*;
pub use filesystem_vod_manager::*;
pub use gcs_vod_manager::*;