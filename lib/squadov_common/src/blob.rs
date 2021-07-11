pub mod aws;
pub mod gcp;

use crate::SquadOvError;
use sqlx::{Transaction, Executor, Postgres};
use uuid::Uuid;
use async_trait::async_trait;

pub enum BlobManagerType {
    GCS
}

pub fn get_blob_manager_type(root: &str) -> BlobManagerType {
    if root.starts_with("gs://") {
        BlobManagerType::GCS
    } else {
        panic!("Unknown blob manager type.");
    }
}

#[async_trait]
pub trait BlobManagementClient {
    async fn store_new_blob(&self, tx : &mut Transaction<'_, Postgres>, bytes: &[u8]) -> Result<Uuid, SquadOvError>;
    async fn get_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<Vec<u8>, SquadOvError>;
    async fn store_new_json_blob(&self, tx : &mut Transaction<'_, Postgres>, val: &serde_json::Value) -> Result<Uuid, SquadOvError>;
    async fn get_json_blob(&self, blob_uuid: &Uuid, is_compressed: bool) -> Result<serde_json::Value, SquadOvError>;
}

pub async fn get_blob_bucket<'a, T>(ex: T, blob_uuid: &Uuid) -> Result<String, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT bucket
            FROM squadov.blob_link_storage
            WHERE uuid = $1
            ",
            blob_uuid
        )
            .fetch_one(ex)
            .await?
            .bucket
    )
}