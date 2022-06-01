use crate::{
    SquadOvError,
};
use rusoto_s3::{
    S3Client,
    S3,
    UploadPartRequest,
    CreateMultipartUploadRequest,
    CompleteMultipartUploadRequest,
    CompletedMultipartUpload,
    CompletedPart,
    PutObjectRequest,
    StreamingBody,
};
use rand::{
    Rng,
    SeedableRng,
};
use md5::Digest;
use tokio::{
    io::{
        AsyncReadExt,
        AsyncSeekExt,
        AsyncRead,
        AsyncSeek,
    },
};

const MULTIPART_SEGMENT_SIZE_BYTES: usize = 100 * 1024 * 1024;

pub async fn s3_multipart_upload_data<T>(s3: &S3Client, mut data: T, total_bytes: usize, bucket: &str, key: &str) -> Result<(), SquadOvError>
where 
    T: AsyncSeek + AsyncRead + std::marker::Unpin
{
    log::info!("Starting Multipart Upload: {}/{} - {} bytes", bucket, key, total_bytes);

    let mut rng = rand::rngs::StdRng::from_entropy();
    if total_bytes == 0 {
        let mut success: bool = false;
        for i in 0u32..5u32 {
            let req = PutObjectRequest{
                bucket: bucket.to_string(),
                key: key.to_string(),
                ..PutObjectRequest::default()
            };

            match s3.put_object(req).await {
                Ok(_) => {
                    success = true;
                    break;
                },
                Err(err) => {
                    log::warn!("Failed to upload empty S3 data: {} - RETRYING", err);
                    async_std::task::sleep(std::time::Duration::from_millis(100u64 * 2u64.pow(i) + rng.gen_range(0..1000))).await;
                    continue;
                }
            }
        }

        if !success {
            return Err(SquadOvError::InternalError(String::from("Failed to Upload Report [0 bytes] - Exceeded retry limit")));
        }
    } else if total_bytes < MULTIPART_SEGMENT_SIZE_BYTES {
        let mut success: bool = false;
        for i in 0u32..5u32 {
            let mut buffer: Vec<u8> = vec![0; total_bytes as usize];
            data.seek(std::io::SeekFrom::Start(0u64)).await?;
            data.read_exact(&mut buffer).await?;

            let md5_hash = {
                let mut hasher = md5::Md5::new();
                hasher.update(&buffer);
                let hash = hasher.finalize();
                base64::encode(hash)
            };

            let req = PutObjectRequest{
                bucket: bucket.to_string(),
                key: key.to_string(),
                body: Some(
                    StreamingBody::from(buffer)
                ),
                content_md5: Some(md5_hash),
                content_type: Some(String::from("application/octet-stream")),
                ..PutObjectRequest::default()
            };

            match s3.put_object(req).await {
                Ok(_) => {
                    success = true;
                    break;
                },
                Err(err) => {
                    log::warn!("Failed to upload single S3 data: {} - RETRYING", err);
                    async_std::task::sleep(std::time::Duration::from_millis(100u64 * 2u64.pow(i) + rng.gen_range(0..1000))).await;
                    continue;
                }
            }
        }

        if !success {
            return Err(SquadOvError::InternalError(String::from("Failed to Upload Report [single] - Exceeded retry limit")));
        }
    } else {
        let upload_id = {
            let req = CreateMultipartUploadRequest{
                bucket: bucket.to_string(),
                key: key.to_string(),
                ..CreateMultipartUploadRequest::default()
            };

            s3.create_multipart_upload(req).await?.upload_id.ok_or(SquadOvError::InternalError(format!("No AWS upload ID returned for multipart upload - {}/{}", bucket, key)))?
        };

        let mut bytes_left_to_upload = total_bytes;
        let num_parts = (bytes_left_to_upload as f32 / MULTIPART_SEGMENT_SIZE_BYTES as f32).ceil() as u64;
        let mut parts: Vec<String> = vec![];
        let mut offset: usize = 0;
        for part in 0..num_parts {
            let mut success = false;
            for i in 0u32..5u32 {
                let part_size_bytes = std::cmp::min(bytes_left_to_upload, MULTIPART_SEGMENT_SIZE_BYTES);

                let mut buffer: Vec<u8> = vec![0; part_size_bytes as usize];
                data.seek(std::io::SeekFrom::Start(offset as u64)).await?;
                data.read_exact(&mut buffer).await?;

                let md5_hash = {
                    let mut hasher = md5::Md5::new();
                    hasher.update(&buffer);
                    let hash = hasher.finalize();
                    base64::encode(hash)
                };

                let req = UploadPartRequest{
                    bucket: bucket.to_string(),
                    key: key.to_string(),
                    part_number: part as i64 + 1,
                    upload_id: upload_id.clone(),
                    body: Some(
                        buffer.into()
                    ),
                    content_md5: Some(md5_hash),
                    content_length: Some(part_size_bytes as i64),
                    ..UploadPartRequest::default()
                };

                let resp = match s3.upload_part(req).await {
                    Ok(r) => r,
                    Err(err) => {
                        log::warn!("Failed to do AWS S3 part upload {:?} - RETRYING", err);
                        async_std::task::sleep(std::time::Duration::from_millis(100u64 * 2u64.pow(i) + rng.gen_range(0..1000))).await;
                        continue;
                    }
                };

                if let Some(e_tag) = resp.e_tag {
                    parts.push(e_tag.clone());
                }

                success = true;
                bytes_left_to_upload -= part_size_bytes;
                offset += part_size_bytes;
                break;
            }

            if !success {
                return Err(SquadOvError::InternalError(String::from("Failed to Upload Report [multi-part] - Exceeded retry limit for a part")));
            }
        }

        let req = CompleteMultipartUploadRequest{
            bucket: bucket.to_string(),
            key: key.to_string(),
            multipart_upload: Some(CompletedMultipartUpload{
                parts: Some(parts.iter().enumerate().map(|(idx, x)| {
                    CompletedPart {
                        e_tag: Some(x.clone()),
                        part_number: Some(idx as i64 + 1),
                    }
                }).collect()),
            }),
            upload_id: upload_id.to_string(),
            ..CompleteMultipartUploadRequest::default()
        };

        s3.complete_multipart_upload(req).await?;
    }
    
    Ok(())
}