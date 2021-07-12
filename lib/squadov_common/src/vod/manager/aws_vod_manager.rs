use crate::{
    SquadOvError,
    VodSegmentId,
    vod::manager::VodManager,
    aws::AWSClient,
};
use std::sync::Arc;
use futures_util::TryStreamExt;
use rusoto_s3::{
    S3,
    StreamingBody,
    GetObjectRequest,
    GetObjectAclRequest,
    PutObjectAclRequest,
    PutObjectRequest,
    DeleteObjectRequest,
    CreateMultipartUploadRequest,
    UploadPartRequest,
    CompleteMultipartUploadRequest, CompletedMultipartUpload, CompletedPart,
    util::{
        PreSignedRequest,
        PreSignedRequestOption,
    }
};
use tokio::fs::{
    File as TFile
};
use tokio_util::codec::{BytesCodec, FramedRead};
use rusoto_credential::ProvideAwsCredentials;
use md5::Digest;

use async_trait::async_trait;
use actix_web::web::{BytesMut};

const S3_URI_PREFIX : &'static str = "s3://";
const S3_ALL_USERS_GROUP: &'static str = "http://acs.amazonaws.com/groups/global/AllUsers";

pub struct S3VodManager {
    bucket: String,
    aws: Arc<Option<AWSClient>>
}

impl S3VodManager {
    pub async fn new(full_bucket: &str, client: Arc<Option<AWSClient>>) -> Result<S3VodManager, SquadOvError> {
        if client.is_none() {
            return Err(SquadOvError::InternalError(String::from("AWS Client not found.")));
        }

        let bucket = full_bucket[S3_URI_PREFIX.len()..].to_string();

        Ok(S3VodManager{
            bucket: bucket.clone(),
            aws: client,
        })
    }

    fn client(&self) -> &AWSClient {
        (*self.aws).as_ref().unwrap()
    }
}

#[async_trait]
impl VodManager for S3VodManager {
    async fn get_segment_redirect_uri(&self, segment: &VodSegmentId) -> Result<String, SquadOvError> {
        let req = GetObjectRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            ..GetObjectRequest::default()
        };

        let creds = self.client().provider.credentials().await?;
        let region = self.client().region.clone();

        Ok(req.get_presigned_url(&region, &creds, &PreSignedRequestOption{
            expires_in: std::time::Duration::from_secs(43200)
        }))
    }

    async fn download_vod_to_path(&self, segment: &VodSegmentId, path: &std::path::Path) -> Result<(), SquadOvError> {
        let req = GetObjectRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            ..GetObjectRequest::default()
        };

        let mut output_file = std::fs::File::create(path)?;
        let resp = (*self.aws).as_ref().unwrap().s3.get_object(req).await?;
        if let Some(body) = resp.body {
            let mut reader = body.into_blocking_read();

            // Stream the download from GCS onto disk so we never have to have to entire file in memory.
            std::io::copy(&mut reader, &mut output_file)?;

            Ok(())
        } else {
            Err(SquadOvError::InternalError(String::from("No VOD downloaded from AWS.")))
        }
    }

    async fn upload_vod_from_file(&self, segment: &VodSegmentId, path: &std::path::Path) -> Result<(), SquadOvError> {
        let md5_hash = {
            let mut file = std::fs::File::open(path)?;
            let mut hasher = md5::Md5::new();
            std::io::copy(&mut file, &mut hasher)?;
            let hash = hasher.finalize();
            base64::encode(hash)
        };

        let file = TFile::open(path).await?;
        let framed_read = FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze);
        
        let req = PutObjectRequest{
            bucket: self.bucket.clone(),
            body: Some(
                StreamingBody::new(framed_read)
            ),
            content_md5: Some(md5_hash),
            content_type: Some(String::from("application/octet-stream")),
            key: segment.get_fname(),
            ..PutObjectRequest::default()
        };
        (*self.aws).as_ref().unwrap().s3.put_object(req).await?;
        Ok(())
    }

    async fn is_vod_session_finished(&self, _session: &str) -> Result<bool, SquadOvError> {
        // No need to check because AWS requires us to finish the upload with a CompleteMultiPartUpload action instead.
        Ok(true)
    }

    async fn start_segment_upload(&self, segment: &VodSegmentId) -> Result<String, SquadOvError> {
        let req = CreateMultipartUploadRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            ..CreateMultipartUploadRequest::default()
        };

        let resp = (*self.aws).as_ref().unwrap().s3.create_multipart_upload(req).await?;
        if let Some(upload_id) = resp.upload_id {
            Ok(upload_id)
        } else {
            Err(SquadOvError::InternalError(String::from("No AWS upload ID returned for multipart upload")))
        }
    }
    
    async fn get_segment_upload_uri(&self, segment: &VodSegmentId, session_id: &str, part: i64) -> Result<String, SquadOvError> {
        let req = UploadPartRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            part_number: part,
            upload_id: session_id.to_string(),
            ..UploadPartRequest::default()
        };

        let creds = self.client().provider.credentials().await?;
        let region = self.client().region.clone();

        Ok(req.get_presigned_url(&region, &creds, &PreSignedRequestOption{
            expires_in: std::time::Duration::from_secs(43200)
        }))
    }

    async fn finish_segment_upload(&self, segment: &VodSegmentId, session_id: &str, parts: &[String]) -> Result<(), SquadOvError> {
        let req = CompleteMultipartUploadRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            multipart_upload: Some(CompletedMultipartUpload{
                parts: Some(parts.iter().enumerate().map(|(idx, x)| {
                    CompletedPart {
                        e_tag: Some(x.clone()),
                        part_number: Some(idx as i64),
                    }
                }).collect()),
            }),
            upload_id: session_id.to_string(),
            ..CompleteMultipartUploadRequest::default()
        };

        (*self.aws).as_ref().unwrap().s3.complete_multipart_upload(req).await?;
        Ok(())
    }

    async fn delete_vod(&self, segment: &VodSegmentId) -> Result<(), SquadOvError> {
        let req = DeleteObjectRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            ..DeleteObjectRequest::default()
        };

        (*self.aws).as_ref().unwrap().s3.delete_object(req).await?;
        Ok(())
    }

    async fn get_public_segment_redirect_uri(&self, segment: &VodSegmentId) -> Result<String, SquadOvError> {
        Ok(
            format!(
                "https://{bucket}.s3.{region}.amazonaws.com/{key}",
                bucket=&self.bucket,
                region=self.client().region.name(),
                key=segment.get_fname(),
            )
        )
    }

    async fn make_segment_public(&self, segment: &VodSegmentId) -> Result<(), SquadOvError> {
        let req = PutObjectAclRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            acl: Some(String::from("public-read")),
            ..PutObjectAclRequest::default()
        };

        self.client().s3.put_object_acl(req).await?;
        Ok(())
    }

    async fn check_vod_segment_is_public(&self, segment: &VodSegmentId) -> Result<bool, SquadOvError> {
        let req = GetObjectAclRequest{
            bucket: self.bucket.clone(),
            key: segment.get_fname(),
            ..GetObjectAclRequest::default()
        };

        let resp = self.client().s3.get_object_acl(req).await?;
        Ok(
            if let Some(grants) = resp.grants {
                let mut is_public = false;
                for g in grants {
                    // We need the "READ" permission.
                    if let Some(permission) = g.permission {
                        if permission != "READ" {
                            continue;
                        }
                    } else {
                        continue;
                    }

                    // Assigned to the All Users grantee
                    if let Some(grantee) = g.grantee {
                        if let Some(uri) = grantee.uri {
                            if uri == S3_ALL_USERS_GROUP {
                                is_public = true;
                                break;
                            }
                        }
                    }
                }

                is_public
            } else {
                false
            }
        )
    }
}