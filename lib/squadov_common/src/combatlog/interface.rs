use crate::{
    SquadOvError,
    aws::AWSClient,
};
use std::sync::Arc;
use rusoto_s3::{
    S3,
    GetObjectRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use avro_rs::{Reader, from_value};

pub struct CombatLogInterface {
    bucket: String,
    aws: Arc<Option<AWSClient>>,
}

impl CombatLogInterface {
    pub fn new(bucket: &str, client: Arc<Option<AWSClient>>) -> Self {
        Self {
            bucket: bucket.to_string(),
            aws: client,
        }
    }

    async fn get_report_raw(&self, key: &str) -> Result<Vec<u8>, SquadOvError> {
        let req = GetObjectRequest{
            bucket: self.bucket.clone(),
            key: key.to_string(),
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

    fn get_key(partition_id: &str, canonical_type: i32, filename: &str) -> String {
        format!("form=Report/partition={}/canonical={}/{}", partition_id, canonical_type, filename)
    }

    pub async fn get_report_json<T>(&self, partition_id: &str, canonical_type: i32, filename: &str) -> Result<T, SquadOvError>
    where
        T: DeserializeOwned
    {
        let key = Self::get_key(partition_id, canonical_type, filename);
        Ok(serde_json::from_slice::<T>(&self.get_report_raw(&key).await?)?)
    }

    pub async fn save_report_json<T>(&self, partition_id: &str, canonical_type: i32, filename: &str, data: T) -> Result<(), SquadOvError>
    where
        T: Serialize
    {
        Ok(())
    }

    pub async fn get_report_avro<T>(&self, partition_id: &str, canonical_type: i32, filename: &str) -> Result<Vec<T>, SquadOvError>
    where
        T: DeserializeOwned
    {
        let key = Self::get_key(partition_id, canonical_type, filename);
        let raw_data = self.get_report_raw(&key).await?;
        if raw_data.is_empty() {
            return Ok(vec![]);
        }
        let reader = Reader::new(&raw_data[..])?;
        let mut ret: Vec<T> = vec![];
        for v in reader {
            ret.push(from_value::<T>(&v?)?);
        }
        Ok(ret)
    }

    pub async fn save_report_avro<T>(&self, partition_id: &str, canonical_type: i32, filename: &str, data: Vec<T>) -> Result<(), SquadOvError>
    where
        T: Serialize
    {
        Ok(())
    }
}