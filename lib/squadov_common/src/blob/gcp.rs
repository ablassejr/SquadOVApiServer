use crate::{
    GCPClient,
    SquadOvError,
    blob::BlobStorageClient,
};
use async_trait::async_trait;
use std::sync::Arc;

const PREFIX : &'static str = "gs://";

pub struct GCPBlobStorage {
    gcp: Arc<Option<GCPClient>>,
}

impl GCPBlobStorage {
    pub fn new(gcp: Arc<Option<GCPClient>>) -> Self {
        if gcp.is_none() {
            panic!("Must supply a GCP client.")
        }

        Self {
            gcp: gcp.clone(),
        }
    }
}

#[async_trait]
impl BlobStorageClient for GCPBlobStorage {
    async fn upload_object(&self, bucket_id: &str, path_parts: &Vec<String>, data: &[u8]) -> Result<(), SquadOvError> {
        Ok((*self.gcp).as_ref().unwrap().gcs().upload_object(bucket_id, path_parts, data).await?)
    }

    async fn download_object(&self, bucket_id: &str, path: &str) -> Result<Vec<u8>, SquadOvError> {
        Ok((*self.gcp).as_ref().unwrap().gcs().download_object(bucket_id, path).await?)
    }

    fn strip_bucket_prefix(&self, bucket: &str) -> String {
        bucket[PREFIX.len()..].to_string()
    }
}