use crate::api::v1;
use crate::common;
use std::sync::Arc;
use std::collections::BTreeMap;

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

    fn get_fname_from_segment_id(&self, segment: &common::VodSegmentId) -> String {
        String::from(vec![segment.video_uuid.to_string(), segment.quality.clone(), segment.segment_name.clone()].join("/"))
    }
}

#[async_trait]
impl v1::VodManager for GCSVodManager {
    async fn get_segment_redirect_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {       
        let fname = self.get_fname_from_segment_id(segment);
        let client = self.get_gcp_client().gcs();

        // Do not insert a check using the Google Cloud Storage API on whether or not the object exists.
        // The GET request will lag behind the user actually finishing the uploading of the object - just
        // give them the URL and if the download fails then oh well.
        client.create_signed_url("GET", &format!("/{}/{}", &self.bucket, fname), &BTreeMap::new())
    }
    
    async fn get_segment_upload_uri(&self, segment: &common::VodSegmentId) -> Result<String, common::SquadOvError> {
        let fname = self.get_fname_from_segment_id(segment);
        let client = self.get_gcp_client().gcs();

        // Unlike redirect function, we actually want the get_object to *fail* to ensure
        // that a video of the same name doesn't already exists in GCS.
        match client.get_object(&self.bucket, &fname).await {
            Ok(_) => return Err(common::SquadOvError::BadRequest),
            Err(err) => match err {
                common::SquadOvError::NotFound => (),
                _ => return Err(err)
            }
        };

        let mut headers = BTreeMap::new();
        headers.insert("x-goog-resumable".to_string(), vec!["start".to_string()]);
        headers.insert("content-type".to_string(), vec!["application/octet-stream".to_string()]);

        client.create_signed_url("POST", &format!("/{}/{}", &self.bucket, fname), &headers)
    }
}