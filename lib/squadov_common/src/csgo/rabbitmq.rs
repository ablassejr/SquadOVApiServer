use async_trait::async_trait;
use std::sync::Arc;
use crate::{
    SquadOvError,
    rabbitmq::{RABBITMQ_DEFAULT_PRIORITY, RabbitMqInterface, RabbitMqListener, RabbitMqConfig},
    csgo::{
        db,
        parser::CsgoDemoParser,
    },
};
use sqlx::postgres::{PgPool};
use serde::{Serialize, Deserialize};
use std::io::{Read};
use uuid::Uuid;
use bzip2::read::BzDecoder;
use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CsgoTask {
    DemoParse {
        url: String,
        view_uuid: Uuid,
        timestamp: DateTime<Utc>,
    }
}

pub struct CsgoRabbitmqInterface {
    mqconfig: RabbitMqConfig,
    rmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
}

impl CsgoRabbitmqInterface {
    pub fn new (mqconfig: &RabbitMqConfig, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>) -> Self {
        Self {
            mqconfig: mqconfig.clone(),
            rmq,
            db,
        }
    }

    pub async fn request_parse_csgo_demo_from_url(&self, view_uuid: &Uuid, demo_url: &str, timestamp: &DateTime<Utc>) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.csgo_queue, serde_json::to_vec(&CsgoTask::DemoParse{
            url: String::from(demo_url),
            view_uuid: view_uuid.clone(),
            timestamp: timestamp.clone(),
        })?, RABBITMQ_DEFAULT_PRIORITY, -1).await;
        Ok(())
    }

    pub async fn parse_csgo_demo_from_bytes(&self, view_uuid: &Uuid, timestamp: &DateTime<Utc>, bytes: &[u8]) -> Result<(), SquadOvError> {
        log::info!("Parsing CSGO demo for View: {}", view_uuid);
        let demo = CsgoDemoParser::from_bytes(bytes)?;
        log::info!("...Finished parsing CSGO demo.");
        let mut tx = self.db.begin().await?;
        db::store_csgo_demo_events_for_view(&mut tx, view_uuid, &demo, timestamp).await?;
        log::info!("...Store CSGO demo in database.");
        tx.commit().await?;
        Ok(())
    }

    pub async fn parse_csgo_demo_from_compressed_local_file(&self, view_uuid: &Uuid, timestamp: &DateTime<Utc>, bytes: &[u8], og_url: &str) -> Result<(), SquadOvError> {
        // Assume that the extension of the demo URL contains the way it was compressed.
        if og_url.ends_with(".dem") {
            self.parse_csgo_demo_from_bytes(view_uuid, timestamp, bytes).await?;
        } else if og_url.ends_with(".bz2") {
            log::info!("Uncompress BZ2 CSGO demo...");
            let mut decoder = BzDecoder::new(bytes);
            let mut buffer: Vec<u8> = vec![];
            decoder.read_to_end(&mut buffer)?;
            self.parse_csgo_demo_from_bytes(view_uuid, timestamp, &buffer).await?;
        } else {
            log::warn!("Failed to recognize CSGO demo compression time: {}", og_url);
            return Err(SquadOvError::BadRequest);
        }
        Ok(())
    }

    pub async fn parse_csgo_demo_from_url(&self, view_uuid: &Uuid, timestamp: &DateTime<Utc>, demo_url: &str) -> Result<(), SquadOvError> {
        let resp = reqwest::get(demo_url).await?;
        let body = resp.bytes().await?;
        self.parse_csgo_demo_from_compressed_local_file(view_uuid, timestamp, &body, demo_url).await?;
        Ok(())
    }
}

#[async_trait]
impl RabbitMqListener for CsgoRabbitmqInterface {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError> {
        log::info!("Handle CSGO RabbitMQ Task: {}", std::str::from_utf8(data).unwrap_or("failure"));
        let task: CsgoTask = serde_json::from_slice(data)?;
        match task {
            CsgoTask::DemoParse{url, view_uuid, timestamp} => self.parse_csgo_demo_from_url(&view_uuid, &timestamp, &url).await?,
        };
        Ok(())
    }
}