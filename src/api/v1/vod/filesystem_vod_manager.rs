use crate::api::v1;
use crate::common;
use async_trait::async_trait;
use uuid::Uuid;
use std::path::Path;
use std::io::Read;
use std::str::FromStr;

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

#[async_trait]
impl v1::VodManager for FilesystemVodManager {
    fn get_segment_redirect_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {
        let fname = Path::new(&self.root).join(&segment.video_uuid.to_string()).join(&segment.quality).join(&segment.segment_name);
        if !fname.exists() {
            return Err(common::SquadOvError::NotFound);
        }

        Ok(String::from_str(fname.to_str().unwrap()).unwrap())
    }
}