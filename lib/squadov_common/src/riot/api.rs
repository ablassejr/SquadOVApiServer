use async_trait::async_trait;

mod account;
mod valorant;

use serde::{Serialize, Deserialize};
use std::sync::Arc;
use crate::{
    SquadOvError,
    rabbitmq::{RabbitMqInterface, RabbitMqListener}
};
use sqlx::postgres::{PgPool};
use reqwest::header;
use tokio::sync::{Semaphore};

#[derive(Deserialize,Debug,Clone)]
pub struct ApiKeyLimit {
    pub requests: usize,
    pub seconds: u64,
}

#[derive(Deserialize,Debug,Clone)]
pub struct RiotApiKeyConfig {
    pub key: String,
    pub burst_limit: ApiKeyLimit,
    pub bulk_limit: ApiKeyLimit,
}

#[derive(Deserialize,Debug,Clone)]
pub struct RiotConfig {
    pub valorant_api_key: RiotApiKeyConfig,
    pub lol_api_key: RiotApiKeyConfig,
    pub tft_api_key: RiotApiKeyConfig,
}

pub struct RiotApiHandler {
    api_key: RiotApiKeyConfig,
    burst_threshold: Arc<Semaphore>,
    bulk_threshold: Arc<Semaphore>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RiotApiTask {
    // puuid
    Account{
        puuid: String
    },
    // puuid
    ValorantBackfill{
        puuid: String
    },
    // match id
    ValorantMatch{
        match_id: String,
        shard: String,
    }
}

impl RiotApiHandler {
    pub fn new(api_key: RiotApiKeyConfig) -> Self {
        let burst_threshold = Arc::new(Semaphore::new(api_key.burst_limit.requests));
        let bulk_threshold = Arc::new(Semaphore::new(api_key.bulk_limit.requests));

        // Spawn two tasks that will handle refreshing the threshold semaphore permit count.
        // We could theoretically have just one task but having two makes the logic much easier.
        {
            let api_key = api_key.clone();
            let burst_threshold = burst_threshold.clone();
            tokio::task::spawn(async move {
                loop {
                    async_std::task::sleep(std::time::Duration::from_secs(api_key.burst_limit.seconds)).await;
                    burst_threshold.add_permits(api_key.burst_limit.requests - burst_threshold.available_permits());
                }
            });
        }

        {
            let api_key = api_key.clone();
            let bulk_threshold = bulk_threshold.clone();
            tokio::task::spawn(async move {
                loop {
                    async_std::task::sleep(std::time::Duration::from_secs(api_key.bulk_limit.seconds)).await;
                    bulk_threshold.add_permits(api_key.bulk_limit.requests - bulk_threshold.available_permits());
                }
            });
        }

        Self {
            api_key,
            burst_threshold,
            bulk_threshold,
        }
    }

    async fn tick_burst_threshold(&self) {
        let permit = self.burst_threshold.acquire().await;
        permit.forget();
    }

    async fn tick_bulk_threshold(&self) {
        let permit = self.bulk_threshold.acquire().await;
        permit.forget();
    }

    async fn tick_thresholds(&self) {
        self.tick_burst_threshold().await;
        self.tick_bulk_threshold().await;
    }

    fn build_api_endpoint(region: &str, endpoint: &str) -> String {
        format!("https://{}.api.riotgames.com/{}", region, endpoint)
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        headers.insert("X-Riot-Token", header::HeaderValue::from_str(&self.api_key.key)?);

        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .build()?)
    }
}

pub struct RiotApiApplicationInterface {
    api: Arc<RiotApiHandler>,
    queue: String,
    rmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
    game: String,
}

impl RiotApiApplicationInterface {
    pub fn new (queue: &str, api: Arc<RiotApiHandler>, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>, game: &str) -> Self {
        Self {
            api,
            queue: String::from(queue),
            rmq,
            db,
            game: String::from(game),
        }
    }
}

#[async_trait]
impl RabbitMqListener for RiotApiApplicationInterface {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError> {
        let task: RiotApiTask = serde_json::from_slice(data)?;
        match task {
            RiotApiTask::Account{puuid} => self.obtain_riot_account_from_puuid(&puuid).await?,
            RiotApiTask::ValorantBackfill{puuid} => self.backfill_user_valorant_matches(&puuid).await?,
            RiotApiTask::ValorantMatch{match_id, shard} => self.obtain_valorant_match_info(&match_id, &shard).await?,
        };
        Ok(())
    }
}