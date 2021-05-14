pub mod filesystem_vod_manager;
pub mod gcs_vod_manager;

pub use filesystem_vod_manager::*;
pub use gcs_vod_manager::*;

use async_trait::async_trait;
use crate::{
    SquadOvError,
    VodSegmentId,
};

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

#[async_trait]
pub trait VodManager {
    async fn get_segment_upload_uri(&self, segment: &VodSegmentId) -> Result<String, SquadOvError>;
    async fn download_vod_to_path(&self, segment: &VodSegmentId, path: &std::path::Path) -> Result<(), SquadOvError>;
    async fn upload_vod_from_file(&self, segment: &VodSegmentId, path: &std::path::Path) -> Result<(), SquadOvError>;
    async fn is_vod_session_finished(&self, session: &str) -> Result<bool, SquadOvError>;
    async fn get_segment_redirect_uri(&self, segment: &VodSegmentId) -> Result<String, SquadOvError>;
    async fn get_public_segment_redirect_uri(&self, segment: &VodSegmentId) -> Result<String, SquadOvError>;
    async fn make_segment_public(&self, segment: &VodSegmentId) -> Result<(), SquadOvError>;
    async fn check_vod_segment_is_public(&self, segment: &VodSegmentId) -> Result<bool, SquadOvError>;
    async fn delete_vod(&self, segment: &VodSegmentId) -> Result<(), SquadOvError>;
}