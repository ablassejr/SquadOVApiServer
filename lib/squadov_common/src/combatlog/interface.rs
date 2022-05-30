use crate::{
    SquadOvError,
    aws::{
        AWSClient,
        s3,
    },
};
use std::sync::Arc;
use rusoto_s3::{
    S3,
    GetObjectRequest,
};
use serde::{Serialize, de::DeserializeOwned};
use avro_rs::{
    Reader,
    from_value,
    Writer,
    Codec,
    Schema,
};
use std::io::{Cursor};

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

    async fn save_report_raw(&self, key: &str, data: Vec<u8>) -> Result<(), SquadOvError> {
        let total_bytes = data.len();
        s3::s3_multipart_upload_data(&(*self.aws).as_ref().unwrap().s3, Cursor::new(data), total_bytes, &self.bucket, key).await?;
        Ok(())
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
        let raw_data = serde_json::to_vec(&data)?;
        self.save_report_raw(&Self::get_key(partition_id, canonical_type, filename), raw_data).await?;
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

    pub async fn save_report_avro<T>(&self, partition_id: &str, canonical_type: i32, filename: &str, schema: &Schema, data: Vec<T>) -> Result<(), SquadOvError>
    where
        T: Serialize
    {
        let mut writer = Writer::with_codec(schema, Vec::new(), Codec::Snappy);
        for d in data {
            writer.append_ser(d)?;
        }

        let raw_data = writer.into_inner()?;
        self.save_report_raw(&Self::get_key(partition_id, canonical_type, filename), raw_data).await?;
        Ok(())
    }
}