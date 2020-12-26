use std::sync::Arc;
use sqlx::postgres::{PgPool};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;
use std::io::Read;
use actix_web::web::{Bytes, BytesMut, BufMut};
use crate::gcp::gcs::GCSUploadStatus;
use chashmap::CHashMap;

pub struct BlobSessionLastByteCache {
    last_byte: CHashMap<Uuid, usize>,
    byte_buffer: CHashMap<Uuid, BytesMut>
}

const BLOB_BYTE_BUFFER_CAPACITY: usize = 8 * 1024 * 1024;

impl BlobSessionLastByteCache {
    fn new() -> Self {
        Self {
            last_byte: CHashMap::new(),
            byte_buffer: CHashMap::new(),
        }
    }

    fn add_bytes_to_buffer(&self, uuid: &Uuid, bytes: &[u8]) -> usize {
        if !self.byte_buffer.contains_key(uuid) {
            self.byte_buffer.insert(uuid.clone(), BytesMut::with_capacity(BLOB_BYTE_BUFFER_CAPACITY));
        }

        let mut buffer = self.byte_buffer.get_mut(uuid).unwrap();
        buffer.put(bytes);
        buffer.len()
    }

    fn obtain_buffer(&self, uuid: &Uuid, split: Option<usize>) -> Bytes {
        if !self.byte_buffer.contains_key(uuid) {
            Bytes::new()
        } else {
            let mut buffer = self.byte_buffer.get_mut(uuid).unwrap();

            match split {
                Some(idx) => buffer.split_to(idx).freeze(),
                None => buffer.split().freeze(),
            }
        }
    }

    fn clear_buffer_for_uuid(&self, uuid: &Uuid) {
        self.byte_buffer.remove(uuid);
    }
    
    fn has_last_byte_for_uuid(&self, uuid: &Uuid) ->  bool {
        self.last_byte.contains_key(uuid)
    }

    fn get_last_byte_for_uuid(&self, uuid: &Uuid) -> Result<usize, crate::SquadOvError> {
        let data = self.last_byte.get(uuid).ok_or(crate::SquadOvError::NotFound)?;
        Ok(*data)
    }

    fn store_last_byte_for_uuid(&self, uuid: &Uuid, last_byte: usize) {
        self.last_byte.insert(uuid.clone(), last_byte);
    }

    fn clear_last_byte_for_uuid(&self, uuid: &Uuid) {
        self.last_byte.remove(uuid);
    }
}

pub struct BlobManagementClient {
    bucket: String,
    gcp: Arc<Option<crate::GCPClient>>,
    db: Arc<PgPool>,
    last_byte_cache: Arc<BlobSessionLastByteCache>
}

pub struct BlobResumableIdentifier {
    pub uuid: Uuid,
    pub session: Option<String>,
}

impl BlobManagementClient {
    pub fn new(gcp: Arc<Option<crate::GCPClient>>, db: Arc<PgPool>) -> Self {
        if gcp.is_none() {
            panic!("Blob management only supports GCS. Must supply a GCP client.")
        }

        let bucket = std::env::var("SQUADOV_BLOB_BUCKET").unwrap();
        BlobManagementClient{
            bucket,
            gcp: gcp.clone(),
            db: db.clone(),
            last_byte_cache: Arc::new(BlobSessionLastByteCache::new())
        }
    }

    pub async fn begin_new_resumable_blob(&self, tx : &mut Transaction<'_, Postgres>) -> Result<BlobResumableIdentifier, crate::SquadOvError> {
        let uuid = Uuid::new_v4();
        let local_path = uuid.to_string();
        let session = (*self.gcp).as_ref().unwrap().gcs().initiate_resumable_upload_session(&self.bucket, &vec![local_path.clone()]).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.blob_link_storage (
                uuid,
                bucket,
                local_path,
                session_uri
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            uuid,
            &self.bucket,
            &local_path,
            &session
        )
            .execute(tx)
            .await?;
        Ok(BlobResumableIdentifier{uuid, session: Some(session)})
    }

    pub async fn store_resumable_blob(&self, id: &BlobResumableIdentifier, bytes: &[u8]) -> Result<(), crate::SquadOvError> {
        if id.session.is_none() {
            return Err(crate::SquadOvError::BadRequest);
        }

        // Add the input bytes to the buffer and only when either
        // 1) The buffer exceeds the upload threshold or
        // 2) The input bytes is empty (no more data)
        // do we actually upload the data to GCS.
        let buffer_size = self.last_byte_cache.add_bytes_to_buffer(&id.uuid, bytes);
        if buffer_size > BLOB_BYTE_BUFFER_CAPACITY || bytes.is_empty() {
            let session = id.session.as_ref().unwrap();
            if !self.last_byte_cache.has_last_byte_for_uuid(&id.uuid) {
                match (*self.gcp).as_ref().unwrap().gcs().get_upload_status(&session).await? {
                    GCSUploadStatus::Complete => return Err(crate::SquadOvError::InternalError(String::from("Attempting to resume a finished GCS streaming upload."))),
                    GCSUploadStatus::Incomplete(byte) => self.last_byte_cache.store_last_byte_for_uuid(&id.uuid, byte),
                }
            }

            let mut last_byte: usize = self.last_byte_cache.get_last_byte_for_uuid(&id.uuid)?;
            let send_buffer = self.last_byte_cache.obtain_buffer(
                &id.uuid,
                if bytes.is_empty() {
                    None
                } else {
                    Some(BLOB_BYTE_BUFFER_CAPACITY)
                }
            );
            let send_buffer_len = send_buffer.len();

            (*self.gcp).as_ref().unwrap().gcs().upload_resumable_object(
                &session,
                last_byte,
                send_buffer,
                bytes.is_empty()
            ).await?;
            last_byte += send_buffer_len;
            self.last_byte_cache.store_last_byte_for_uuid(&id.uuid, last_byte);
        }
        
        Ok(())
    }

    pub async fn finish_resumable_blob(&self, id: &BlobResumableIdentifier) -> Result<(), crate::SquadOvError> {
        self.store_resumable_blob(id, &vec![]).await?;
        self.last_byte_cache.clear_last_byte_for_uuid(&id.uuid);
        self.last_byte_cache.clear_buffer_for_uuid(&id.uuid);
        Ok(())
    }

    pub async fn store_new_blob(&self, tx : &mut Transaction<'_, Postgres>, bytes: &[u8]) -> Result<Uuid, crate::SquadOvError> {
        // Let's assume that blobs are large enough for compresssion to make a difference.
        let mut compressed_bytes: Vec<u8> = Vec::new();
        {
            // A quality of 6 seems to be a good balanace between size and speed.
            let mut compressor = brotli2::read::BrotliEncoder::new(bytes, 6);
            compressor.read_to_end(&mut compressed_bytes)?;
        }

        let uuid = Uuid::new_v4();
        let local_path = uuid.to_string();
        sqlx::query!(
            "
            INSERT INTO squadov.blob_link_storage (
                uuid,
                bucket,
                local_path
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ",
            uuid,
            &self.bucket,
            &local_path,
        )
            .execute(tx)
            .await?;

        (*self.gcp).as_ref().unwrap().gcs().upload_object(&self.bucket, &vec![local_path.clone()], &compressed_bytes).await?;
        Ok(uuid)
    }

    pub async fn get_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<Vec<u8>, crate::SquadOvError> {
        let data = sqlx::query!(
            "
            SELECT bucket, local_path
            FROM squadov.blob_link_storage
            WHERE uuid = $1
            ",
            blob_uuid
        )
            .fetch_optional(&*self.db)
            .await?;
        
        if data.is_none() {
            return Err(crate::SquadOvError::NotFound)
        }

        let data = data.unwrap();
        let compressed_bytes = (*self.gcp).as_ref().unwrap().gcs().download_object(&data.bucket, &data.local_path).await?;

        if is_compressed {
            let mut uncompressed_bytes: Vec<u8> = Vec::new();
            {
                let mut decompressor = brotli2::read::BrotliDecoder::new(&compressed_bytes[..]);
                decompressor.read_to_end(&mut uncompressed_bytes)?;
            }
            Ok(uncompressed_bytes)
        } else {
            Ok(compressed_bytes)
        }
    }

    pub async fn store_new_json_blob(&self, tx : &mut Transaction<'_, Postgres>, val: &serde_json::Value) -> Result<Uuid, crate::SquadOvError> {
        self.store_new_blob(tx, &serde_json::to_vec(val)?).await
    }

    pub async fn get_json_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<serde_json::Value, crate::SquadOvError> {
        let blob = self.get_blob(blob_uuid, is_compressed).await?;
        let value = serde_json::from_slice(&blob)?;
        Ok(value)
    }
}