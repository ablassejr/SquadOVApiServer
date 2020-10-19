use async_trait::async_trait;
use crate::common;
use uuid::Uuid;

pub struct FilesystemVodManager {
    
}

#[async_trait]
impl super::VodManager for FilesystemVodManager {
    async fn get_vod_video_quality_options(&self, video_uuid: &Uuid) -> Result<Vec<super::VodVideoQualityOption>, common::SquadOvError> {
        let ret = vec![];
        Ok(ret)
    }
}