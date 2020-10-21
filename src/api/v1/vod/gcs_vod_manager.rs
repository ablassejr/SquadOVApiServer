use crate::api::v1;
use crate::common;
use std::sync::Arc;

use async_trait::async_trait;

const GS_URI_PREFIX : &'static str = "gs://";

pub struct GCSVodManager {
    bucket: String,
    client: Arc<Option<common::GCPClient>>
}

impl GCSVodManager {
    pub async fn new(client: Arc<Option<common::GCPClient>>) -> Result<GCSVodManager, common::SquadOvError> {
        let uri = std::env::var("SQUADOV_VOD_ROOT").unwrap();

        if client.is_none() {
            return Err(common::SquadOvError::InternalError(String::from("GCP Client not found.")));
        }

        let bucket = uri[GS_URI_PREFIX.len()..].to_string();

        // Do a sanity check to make sure the bucket exists to protect against dev typos!!!!
        // If this fails we'll force panic and fail
        client.as_ref().as_ref().unwrap().gcs().get_bucket(&bucket).await?;

        Ok(GCSVodManager{
            bucket: bucket.clone(),
            client: client,
        })
    }

    fn get_gcp_client(&self) -> &common::GCPClient {
        (*self.client).as_ref().unwrap()
    }
}

#[async_trait]
impl v1::VodManager for GCSVodManager {
    async fn get_segment_redirect_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {       
        let fname = vec![segment.video_uuid.to_string(), segment.quality.clone(), segment.segment_name.clone()].join("/");
        let client = self.get_gcp_client().gcs();

        // Make sure it exists so we can give the user a failure message here if it doesn't actually exist
        // before they go and try to pull from the signed uRL.
        client.get_object(&self.bucket, &fname).await?;

        client.create_signed_url(&self.bucket, &fname)
    }
}