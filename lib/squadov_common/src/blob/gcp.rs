use crate::{
    GCPClient,
    SquadOvError,
    blob::BlobManagementClient,
};
use sqlx::postgres::{PgPool};
use sqlx::{Transaction, Postgres};
use async_trait::async_trait;
use std::sync::Arc;
use std::io::Read;
use uuid::Uuid;

pub struct GCPBlobManager {
    bucket: String,
    gcp: Arc<Option<GCPClient>>,
    db: Arc<PgPool>,
}

impl GCPBlobManager {
    pub fn new(bucket: &str, gcp: Arc<Option<GCPClient>>, db: Arc<PgPool>) -> Self {
        if gcp.is_none() {
            panic!("Must supply a GCP client.")
        }

        Self {
            bucket: bucket.to_string(),
            gcp: gcp.clone(),
            db: db.clone(),
        }
    }
}

#[async_trait]
impl BlobManagementClient for GCPBlobManager {
    async fn store_new_blob(&self, tx : &mut Transaction<'_, Postgres>, bytes: &[u8]) -> Result<Uuid, SquadOvError> {
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

    async fn get_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<Vec<u8>, SquadOvError> {
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

    async fn store_new_json_blob(&self, tx : &mut Transaction<'_, Postgres>, val: &serde_json::Value) -> Result<Uuid, SquadOvError> {
        self.store_new_blob(tx, &serde_json::to_vec(val)?).await
    }

    async fn get_json_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<serde_json::Value, SquadOvError> {
        let blob = self.get_blob(blob_uuid, is_compressed).await?;
        let value = serde_json::from_slice(&blob)?;
        Ok(value)
    }
}