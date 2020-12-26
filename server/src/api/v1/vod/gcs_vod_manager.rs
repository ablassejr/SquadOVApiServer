use crate::api::v1;
use squadov_common;
use squadov_common::gcp::gcs::GCSUploadStatus;
use std::sync::Arc;
use std::collections::BTreeMap;
use std::io::Write;
use std::fs::File;
use std::io::Read;

use async_trait::async_trait;
use rand::Rng;
use actix_web::web::{BytesMut, Bytes};

const GS_URI_PREFIX : &'static str = "gs://";
const MAX_GCS_RETRIES: i32 = 10;
const MAX_GCS_BACKOFF_TIME_MS: i64 = 32000;

pub struct GCSVodManager {
    bucket: String,
    client: Arc<Option<squadov_common::GCPClient>>
}

impl GCSVodManager {
    pub async fn new(client: Arc<Option<squadov_common::GCPClient>>) -> Result<GCSVodManager, squadov_common::SquadOvError> {
        let uri = std::env::var("SQUADOV_VOD_ROOT").unwrap();

        if client.is_none() {
            return Err(squadov_common::SquadOvError::InternalError(String::from("GCP Client not found.")));
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

    fn get_gcp_client(&self) -> &squadov_common::GCPClient {
        (*self.client).as_ref().unwrap()
    }

    fn get_path_parts_from_segment_id(&self, segment: &squadov_common::VodSegmentId) -> Vec<String> {
        vec![segment.video_uuid.to_string(), segment.quality.clone(), segment.segment_name.clone()]
    }

    fn get_fname_from_segment_id(&self, segment: &squadov_common::VodSegmentId) -> String {
        self.get_path_parts_from_segment_id(segment).join("/")
    }
}

#[async_trait]
impl v1::VodManager for GCSVodManager {
    async fn get_segment_redirect_uri(&self, segment: &squadov_common::VodSegmentId) -> Result<String, squadov_common::SquadOvError> {       
        let fname = self.get_fname_from_segment_id(segment);
        let client = self.get_gcp_client().gcs();

        // Do not insert a check using the Google Cloud Storage API on whether or not the object exists.
        // The GET request will lag behind the user actually finishing the uploading of the object - just
        // give them the URL and if the download fails then oh well.
        client.create_signed_url("GET", &format!("/{}/{}", &self.bucket, fname), &BTreeMap::new())
    }

    async fn download_vod_to_path(&self, segment: &squadov_common::VodSegmentId, path: &std::path::Path) -> Result<(), squadov_common::SquadOvError> {
        let uri = self.get_segment_redirect_uri(segment).await?;
        let resp = reqwest::get(&uri).await?;
        let mut output_file = std::fs::File::create(path)?;
        let body = resp.bytes().await?;
        output_file.write_all(&body)?;
        Ok(())
    }

    async fn upload_vod_from_file(&self, segment: &squadov_common::VodSegmentId, path: &std::path::Path) -> Result<(), squadov_common::SquadOvError> {
        let client = self.get_gcp_client().gcs();
        let fname = self.get_path_parts_from_segment_id(segment);
        
        // Stream from the file to GCS.
        let mut f = File::open(path)?;
        let mut buffer = BytesMut::with_capacity(8 * 1024 * 1024);
        buffer.resize(8 * 1024 * 1024, 0);

        log::info!("Start resumable upload session: {:?}", segment);
        let session = client.initiate_resumable_upload_session(&self.bucket, &fname).await?;
        log::info!("Obtained resumable upload session: {:?} :: {}", segment, session);
        let mut total_bytes: usize = 0;
        loop {
            let n = f.read(&mut buffer)?;
            let last = n < buffer.len();
            let mut success = false;
            for i in 0..MAX_GCS_RETRIES {
                let byte_buffer = Bytes::copy_from_slice(&buffer[0..n]);
                match client.upload_resumable_object(&session, total_bytes, byte_buffer, last).await {
                    Ok(_) => {
                        success = true;
                        break;
                    },
                    Err(err) => {
                        let backoff_ms = {
                            let mut rng = rand::thread_rng();
                            std::cmp::min(2i64.pow(i as u32) +  rng.gen_range(0..1000), MAX_GCS_BACKOFF_TIME_MS)
                        };
                        log::warn!("Failed to upload to GCS: {:?} - Retrying {} @{}ms", err, i, backoff_ms);
                        async_std::task::sleep(std::time::Duration::from_millis(backoff_ms as u64)).await;
                    }
                }
            }

            if !success {
                return Err(squadov_common::SquadOvError::InternalError(String::from("Max GCS upload retry limit exceeded.")));
            }
            
            total_bytes += n;
            if last {
                break;
            }
        }

        Ok(())
    }

    async fn is_vod_session_finished(&self, session: &str) -> Result<bool, squadov_common::SquadOvError> {
        let client = self.get_gcp_client().gcs();
        Ok(client.get_upload_status(session).await? == GCSUploadStatus::Complete)
    }
    
    async fn get_segment_upload_uri(&self, segment: &squadov_common::VodSegmentId) -> Result<String, squadov_common::SquadOvError> {
        let fname = self.get_fname_from_segment_id(segment);
        let client = self.get_gcp_client().gcs();

        // Unlike redirect function, we actually want the get_object to *fail* to ensure
        // that a video of the same name doesn't already exists in GCS.
        match client.get_object(&self.bucket, &fname).await {
            Ok(_) => return Err(squadov_common::SquadOvError::BadRequest),
            Err(err) => match err {
                squadov_common::SquadOvError::NotFound => (),
                _ => return Err(err)
            }
        };

        let mut headers = BTreeMap::new();
        headers.insert("x-goog-resumable".to_string(), vec!["start".to_string()]);
        headers.insert("content-type".to_string(), vec!["application/octet-stream".to_string()]);

        client.create_signed_url("POST", &format!("/{}/{}", &self.bucket, fname), &headers)
    }

    async fn delete_vod(&self, segment: &squadov_common::VodSegmentId) -> Result<(), squadov_common::SquadOvError> {
        let fname = self.get_fname_from_segment_id(segment);
        let client = self.get_gcp_client().gcs();
        Ok(client.delete_object(&self.bucket, &fname).await?)
    }
}