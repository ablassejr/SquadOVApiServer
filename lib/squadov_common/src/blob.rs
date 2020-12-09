use std::sync::Arc;
use sqlx::postgres::{PgPool};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;
use std::io::Read;

pub struct BlobManagementClient {
    bucket: String,
    gcp: Arc<Option<crate::GCPClient>>,
    db: Arc<PgPool>
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
        }
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

        (*self.gcp).as_ref().unwrap().gcs().upload_object(&self.bucket, &local_path, &compressed_bytes).await?;
        Ok(uuid)
    }

    pub async fn get_blob(&self, blob_uuid: &Uuid) -> Result<Vec<u8>, crate::SquadOvError> {
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

        let mut uncompressed_bytes: Vec<u8> = Vec::new();
        {
            let mut decompressor = brotli2::read::BrotliDecoder::new(&compressed_bytes[..]);
            decompressor.read_to_end(&mut uncompressed_bytes)?;
        }
        Ok(uncompressed_bytes)
    }

    pub async fn store_new_json_blob(&self, tx : &mut Transaction<'_, Postgres>, val: &serde_json::Value) -> Result<Uuid, crate::SquadOvError> {
        self.store_new_blob(tx, &serde_json::to_vec(val)?).await
    }

    pub async fn get_json_blob(&self, blob_uuid: &Uuid) -> Result<serde_json::Value, crate::SquadOvError> {
        let timer = std::time::Instant::now();
        let blob = self.get_blob(blob_uuid).await?;
        println!("\tBLOB DOWNLOAD: {:?}", timer.elapsed());
        let value = serde_json::from_slice(&blob)?;
        println!("\tJSON FROM SLICE: {:?}", timer.elapsed());
        Ok(value)
    }
}