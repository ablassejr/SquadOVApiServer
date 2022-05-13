pub mod io;
pub mod agg;
pub mod db;

use crate::{
    SquadOvError,
    aws::s3,
};
use rusoto_s3::{
    S3Client,
};
use std::sync::Arc;
use tokio::{
    fs::File,
};

use serde::{de::DeserializeOwned, Serialize, Deserialize};
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use async_trait::async_trait;
use async_std::sync::{RwLock};
use sqlx::{
    {Transaction, Postgres},
};
use std::ops::DerefMut;

pub const LOG_FLUSH: &'static str = "//SQUADOV_COMBAT_LOG_FLUSH";

#[derive(Deserialize, Clone)]
pub struct CombatLog {
    pub partition_id: String,
    pub start_time: DateTime<Utc>,
    pub owner_id: i64,
    pub cl_state: serde_json::Value,
}

#[derive(PartialEq)]
pub enum CombatLogReportType {
    Static,
    Dynamic
}

#[async_trait]
pub trait CombatLogReport {
    fn report_type(&self) -> CombatLogReportType;
    async fn store_static_report(&self, bucket: String, partition: String, s3: Arc<S3Client>) -> Result<(), SquadOvError>;
    async fn store_dynamic_report(&self, tx: &mut Transaction<'_, Postgres>) -> Result<(), SquadOvError>;
}

pub struct RawStaticCombatLogReport {
    // Name of the file we store into S3.
    pub key_name: String,
    // The file that contains the data on disk.
    pub raw_file: RwLock<File>,
    // The 'type' of the report. This number depends on the game we're generating the report for.
    pub canonical_type: i32,
}

#[async_trait]
impl CombatLogReport for RawStaticCombatLogReport {
    fn report_type(&self) -> CombatLogReportType {
        CombatLogReportType::Static
    }

    async fn store_static_report(&self, bucket: String, partition: String, s3: Arc<S3Client>) -> Result<(), SquadOvError> {
        store_single_static_report(self, &bucket, &partition, s3).await
    }

    async fn store_dynamic_report(&self, _tx: &mut Transaction<'_, Postgres>) -> Result<(), SquadOvError> {
        Err(SquadOvError::BadRequest)
    }
}

pub trait CombatLogReportHandler {
    type Data;

    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError>;
}

pub trait CombatLogReportIO {
    fn finalize(&mut self) -> Result<(), SquadOvError>;
    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError>;
    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError>;
}

pub trait CombatLogReportParser {
    fn handle(&mut self, data: &str) -> Result<(), SquadOvError>;
}

pub trait CombatLogReportGenerator: CombatLogReportParser + CombatLogReportIO {}

pub struct CombatLogReportContainer<T> {
    generator: T,
}

impl<T> CombatLogReportGenerator for CombatLogReportContainer<T>
where
    T: CombatLogReportHandler + CombatLogReportIO,
    T::Data: DeserializeOwned,
{}

impl<T> CombatLogReportParser for CombatLogReportContainer<T>
where
    T: CombatLogReportHandler,
    T::Data: DeserializeOwned,
{
    fn handle(&mut self, data: &str) -> Result<(), SquadOvError> {
        let data = serde_json::from_str::<T::Data>(data)?;
        self.generator.handle(&data)?;
        Ok(())
    }
}

impl<T> CombatLogReportIO for CombatLogReportContainer<T>
where
    T: CombatLogReportIO
{
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        self.generator.finalize()
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.generator.initialize_work_dir(dir)
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        self.generator.get_reports()
    }
}

impl<T> CombatLogReportContainer<T> {
    pub fn new(generator: T) -> Self {
        Self {
            generator,
        }
    }
}

pub trait CombatLogPacket {
    type Data : Clone + Serialize + Debug;

    fn parse_from_raw(partition_key: String, raw: String, cl_state: serde_json::Value) -> Result<Option<Self::Data>, SquadOvError>;
    fn create_flush_packet(partition_key: String) -> Self::Data;
    fn create_raw_packet(partition_key: String, tm: DateTime<Utc>, raw: String) -> Self::Data;
    fn extract_timestamp(data: &Self::Data) -> DateTime<Utc>;
}

async fn store_single_static_report(report: &RawStaticCombatLogReport, bucket: &str, partition: &str, s3_client: Arc<S3Client>) -> Result<(), SquadOvError> {
    let key = format!(
        "form=Report/partition={partition}/canonical={canonical}/{name}",
        partition=partition,
        canonical=report.canonical_type,
        name=&report.key_name,
    );

    let mut raw_file = report.raw_file.write().await;
    let byte_size = raw_file.metadata().await?.len() as usize;
    s3::s3_multipart_upload_data(s3_client, raw_file.deref_mut(), byte_size, bucket, &key).await?;
    Ok(())
}

pub async fn store_static_combat_log_reports<'a>(reports: Vec<Arc<dyn CombatLogReport + Send + Sync>>, bucket: &'a str, partition: &'a str, s3: Arc<S3Client>) -> Result<(), SquadOvError> {
    let handles = reports.into_iter()
        .filter(|x| { x.report_type() == CombatLogReportType::Static })
        .map(|report| {
            let bucket = String::from(bucket);
            let partition = String::from(partition);
            let s3 = s3.clone();
            tokio::task::spawn(async move {
                report.store_static_report(bucket, partition, s3).await?;
                Ok::<(), SquadOvError>(())
            })
        })
        .collect::<Vec<_>>();

    for h in handles {
        h.await??;
    }
    Ok(())
}