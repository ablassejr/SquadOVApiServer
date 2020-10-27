use crate::api::v1;
use crate::common;
use async_trait::async_trait;
use std::path::{Path,PathBuf};
use std::fs;

pub struct FilesystemVodManager {
    root: String
}

impl FilesystemVodManager {
    pub fn new() -> Result<FilesystemVodManager, common::SquadOvError> {
        let manager = FilesystemVodManager{
            root: std::env::var("SQUADOV_VOD_ROOT").unwrap()
        };

        // The root folder must exist.
        if !Path::new(&manager.root).exists() {
            return Err(common::SquadOvError::NotFound);
        }

        Ok(manager)
    }
}

impl FilesystemVodManager {
    fn segment_id_to_path(&self, segment: &common::VodSegmentId) -> PathBuf {
        Path::new(&self.root).join(&segment.video_uuid.to_string()).join(&segment.quality).join(&segment.segment_name)
    }
}

#[async_trait]
impl v1::VodManager for FilesystemVodManager {
    async fn get_segment_redirect_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {
        let fname = self.segment_id_to_path(segment);
        if !fname.exists() {
            return Err(common::SquadOvError::NotFound);
        }

        Ok(String::from(fname.to_str().unwrap_or("")))
    }

    async fn get_segment_upload_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {
        Ok(String::from(self.segment_id_to_path(segment).to_str().unwrap_or("")))
    }

    async fn delete_vod(&self, segment: &common::VodSegmentId) -> Result<(), common::SquadOvError> {
        let fname = self.segment_id_to_path(segment);
        if !fname.exists() {
            return Err(common::SquadOvError::NotFound);
        }

        Ok(fs::remove_file(fname)?)
    }
}