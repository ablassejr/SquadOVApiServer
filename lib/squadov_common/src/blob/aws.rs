use crate::{
    SquadOvError,
    blob::BlobStorageClient,
    aws::AWSClient,
};
use async_trait::async_trait;
use std::sync::Arc;
use rusoto_s3::{
    S3,
    StreamingBody,
    GetObjectRequest,
    PutObjectRequest
};
use md5::Digest;

const PREFIX : &'static str = "s3://";

pub struct AWSBlobStorage {
    aws: Arc<Option<AWSClient>>,
}

impl AWSBlobStorage {
    pub fn new(aws: Arc<Option<AWSClient>>) -> Self {
        if aws.is_none() {
            panic!("Must supply a AWS client.")
        }

        Self {
            aws: aws.clone(),
        }
    }
}

#[async_trait]
impl BlobStorageClient for AWSBlobStorage {
    async fn upload_object(&self, bucket_id: &str, path_parts: &Vec<String>, data: &[u8]) -> Result<(), SquadOvError> {
        let req = PutObjectRequest{
            bucket: bucket_id.to_string(),
            body: Some(
                StreamingBody::from(data.iter().map(|x| {*x}).collect::<Vec<u8>>())
            ),
            content_md5: Some(base64::encode(md5::Md5::digest(data))),
            content_type: Some(String::from("application/octet-stream")),
            key: path_parts.join("/"),
            ..PutObjectRequest::default()
        };
        (*self.aws).as_ref().unwrap().s3.put_object(req).await?;
        Ok(())
    }

    async fn download_object(&self, bucket_id: &str, path: &str) -> Result<Vec<u8>, SquadOvError> {
        let req = GetObjectRequest{
            bucket: bucket_id.to_string(),
            key: path.to_string(),
            ..GetObjectRequest::default()
        };

        let result = (*self.aws).as_ref().unwrap().s3.get_object(req).await?;

        if let Some(body) = result.body {
            let mut reader = body.into_async_read();

            let mut bytes: Vec<u8> = Vec::new();
            tokio::io::copy(&mut reader, &mut bytes).await?;

            Ok(bytes)
        } else {
            Ok(vec![])
        }
    }

    fn strip_bucket_prefix(&self, bucket: &str) -> String {
        bucket[PREFIX.len()..].to_string()
    }
}