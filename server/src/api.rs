pub mod api_service;
pub mod auth;
pub mod fusionauth;
pub mod access;
pub mod v1;
pub mod graphql;
pub mod admin;
pub mod oembed;

use serde::{Deserialize};
use sqlx::postgres::{PgPool};
use actix_web::{HttpRequest};
use squadov_common;
use squadov_common::{
    SquadOvError,
    HalResponse,
    BlobManagementClient,
    KafkaCredentialKeyPair,
    riot::{
        api::{RiotApiHandler, RiotApiApplicationInterface, RiotConfig},
    },
    rabbitmq::{RabbitMqInterface, RabbitMqConfig},
    EmailConfig,
    EmailClient,
    vod,
    vod::VodProcessingInterface,
    vod::manager::{
        VodManagerType,
        VodManager,
        GCSVodManager,
        FilesystemVodManager,
    }
};
use url::Url;
use std::vec::Vec;
use std::sync::Arc;
use sqlx::postgres::{PgPoolOptions};

// TODO: REMOVE THIS.
#[macro_export]
macro_rules! logged_error {
    ($x:expr) => {{
        warn!("{}", $x); Err($x)
    }};
}


#[derive(Deserialize)]
pub struct PaginationParameters {
    pub start: i64,
    pub end: i64
}

fn replace_pagination_parameters_in_url(url: &str, start : i64, end : i64) -> Result<String, SquadOvError> {
    let mut url = Url::parse(url)?;
    let mut query_params: Vec<(String, String)> = url.query_pairs().into_owned().collect();

    {
        let mut new_params = url.query_pairs_mut();
        new_params.clear();

        for pair in &mut query_params {
            if pair.0 == "start" {
                pair.1 = format!("{}", start);
            } else if pair.0 == "end" {
                pair.1 = format!("{}", end);
            }
            new_params.append_pair(&pair.0, &pair.1);
        }
    }

    Ok(String::from(url.as_str()))
}

pub fn construct_hal_pagination_response<T>(data : T, req: &HttpRequest, params: &PaginationParameters, has_next: bool) -> Result<HalResponse<T>, SquadOvError> {
    let conn = req.connection_info();
    let raw_url = format!("{}://{}{}", conn.scheme(), conn.host(), req.uri().to_string());
    let count = params.end - params.start;

    let mut response = HalResponse::new(data);
    response.add_link("self", &raw_url);

    if has_next {
        let next_start = params.end;
        let next_end = params.end + count;
        response.add_link("next", &replace_pagination_parameters_in_url(&raw_url, next_start, next_end)?);
    }

    if params.start != 0 {
        let prev_start = params.start - count;
        let prev_end = params.start;
        response.add_link("prev", &replace_pagination_parameters_in_url(&raw_url, prev_start, prev_end)?);
    }

    Ok(response)
}

#[derive(Deserialize,Debug,Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub connections: u32,
    pub heavy_connections: u32,
}

#[derive(Deserialize,Debug,Clone)]
pub struct CorsConfig {
    pub domain: String
}

#[derive(Deserialize,Debug,Clone)]
pub struct ServerConfig {
    pub domain: String,
    pub graphql_debug: bool
}

#[derive(Deserialize,Debug,Clone)]
pub struct GitlabConfig {
    pub access_token: String,
    pub project_id: u64
}

#[derive(Deserialize,Debug,Clone)]
pub struct VodConfig {
    pub fastify_threads: i32
}

#[derive(Deserialize,Debug,Clone)]
pub struct KafkaConfig {
    pub bootstrap_servers: String,
    pub wow_combat_log_threads: i32,
    pub wow_combat_log_topic: String,
    pub client_keypair: KafkaCredentialKeyPair,
    pub server_keypair: KafkaCredentialKeyPair
}

#[derive(Deserialize,Debug,Clone)]
pub struct SquadOvConfig {
    pub app_url: String,
    pub landing_url: String,
    pub invite_key: String,
    pub share_key: String,
}

#[derive(Deserialize,Debug,Clone)]
pub struct ApiConfig {
    fusionauth: fusionauth::FusionAuthConfig,
    pub gcp: squadov_common::GCPConfig,
    pub database: DatabaseConfig,
    pub cors: CorsConfig,
    pub server: ServerConfig,
    pub gitlab: GitlabConfig,
    pub kafka: KafkaConfig,
    pub vod: VodConfig,
    pub riot: RiotConfig,
    pub rabbitmq: RabbitMqConfig,
    pub email: EmailConfig,
    pub squadov: SquadOvConfig,
}

pub struct ApiClients {
    pub fusionauth: fusionauth::FusionAuthClient,
}

pub struct ApiApplication {
    pub config: ApiConfig,
    pub clients: ApiClients,
    pub users: auth::UserManager,
    session: auth::SessionManager,
    vod: Arc<dyn VodManager + Send + Sync>,
    pub pool: Arc<PgPool>,
    pub heavy_pool: Arc<PgPool>,
    schema: Arc<graphql::GraphqlSchema>,
    pub blob: Arc<BlobManagementClient>,
    pub rso_itf: Arc<RiotApiApplicationInterface>,
    pub valorant_itf: Arc<RiotApiApplicationInterface>,
    pub lol_itf: Arc<RiotApiApplicationInterface>,
    pub tft_itf: Arc<RiotApiApplicationInterface>,
    pub email: Arc<EmailClient>,
    pub vod_itf: Arc<VodProcessingInterface>,
}

impl ApiApplication {
    pub async fn new(config: &ApiConfig) -> ApiApplication {
        let disable_rabbitmq = std::env::var("DISABLE_RABBITMQ").is_ok();

        // Use TOML config to create application - e.g. for
        // database configuration, external API client configuration, etc.
        let pool = Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(config.database.connections)
            .max_lifetime(std::time::Duration::from_secs(6*60*60))
            .idle_timeout(std::time::Duration::from_secs(3*60*60))
            .connect(&config.database.url)
            .await
            .unwrap());

        let heavy_pool = Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(config.database.heavy_connections)
            .max_lifetime(std::time::Duration::from_secs(3*60*60))
            .idle_timeout(std::time::Duration::from_secs(1*60*60))
            .connect(&config.database.url)
            .await
            .unwrap());

        let gcp = Arc::new(
            if config.gcp.enabled {
                Some(squadov_common::GCPClient::new(&config.gcp).await)
            } else {
                None
            }
        );

        let blob = Arc::new(BlobManagementClient::new(gcp.clone(), pool.clone()));
        
        let rso_api = Arc::new(RiotApiHandler::new(config.riot.rso_api_key.clone()));
        let valorant_api = Arc::new(RiotApiHandler::new(config.riot.valorant_api_key.clone()));
        let lol_api = Arc::new(RiotApiHandler::new(config.riot.lol_api_key.clone()));
        let tft_api = Arc::new(RiotApiHandler::new(config.riot.tft_api_key.clone()));
        let rabbitmq = RabbitMqInterface::new(&config.rabbitmq, pool.clone(), !disable_rabbitmq).await.unwrap();

        let rso_itf = Arc::new(RiotApiApplicationInterface::new(config.riot.clone(), &config.rabbitmq, rso_api.clone(), rabbitmq.clone(), pool.clone()));
        let valorant_itf = Arc::new(RiotApiApplicationInterface::new(config.riot.clone(), &config.rabbitmq, valorant_api.clone(), rabbitmq.clone(), pool.clone()));
        let lol_itf = Arc::new(RiotApiApplicationInterface::new(config.riot.clone(), &config.rabbitmq, lol_api.clone(), rabbitmq.clone(), pool.clone()));
        let tft_itf = Arc::new(RiotApiApplicationInterface::new(config.riot.clone(), &config.rabbitmq, tft_api.clone(), rabbitmq.clone(), pool.clone()));

        if !disable_rabbitmq {
            if config.rabbitmq.enable_rso {
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.rso_queue.clone(), rso_itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
            }

            if config.rabbitmq.enable_valorant {
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.valorant_queue.clone(), valorant_itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
            }

            if config.rabbitmq.enable_lol {
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.lol_queue.clone(), lol_itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
            }

            if config.rabbitmq.enable_tft {
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.tft_queue.clone(), tft_itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
            }
        }

        let vod_manager = match vod::manager::get_current_vod_manager_type() {
            VodManagerType::GCS => Arc::new(GCSVodManager::new(gcp.clone()).await.unwrap()) as Arc<dyn VodManager + Send + Sync>,
            VodManagerType::FileSystem => Arc::new(FilesystemVodManager::new().unwrap()) as Arc<dyn VodManager + Send + Sync>
        };

        // One VOD interface for publishing - individual interfaces for consuming.
        let vod_itf = Arc::new(VodProcessingInterface::new(&config.rabbitmq.vod_queue, rabbitmq.clone(), pool.clone(), vod_manager.clone()));
        if !disable_rabbitmq && config.rabbitmq.enable_vod {
            for _i in 0..config.vod.fastify_threads {
                let process_itf = Arc::new(VodProcessingInterface::new(&config.rabbitmq.vod_queue, rabbitmq.clone(), pool.clone(), vod_manager.clone()));
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.vod_queue.clone(), process_itf, config.rabbitmq.prefetch_count).await.unwrap();
            }
        }

        ApiApplication{
            config: config.clone(),
            clients: ApiClients{
                fusionauth: fusionauth::FusionAuthClient::new(config.fusionauth.clone()),
            },
            users: auth::UserManager{},
            session: auth::SessionManager::new(),
            vod: vod_manager,
            pool,
            heavy_pool,
            schema: Arc::new(graphql::create_schema()),
            blob: blob,
            rso_itf,
            valorant_itf,
            lol_itf,
            tft_itf,
            email: Arc::new(EmailClient::new(&config.email)),
            vod_itf,
        }
    }
}