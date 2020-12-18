pub mod api_service;
pub mod auth;
pub mod fusionauth;
pub mod access;
pub mod v1;
pub mod graphql;

use serde::{Deserialize};
use sqlx::postgres::{PgPool};
use actix_web::{HttpRequest};
use squadov_common;
use squadov_common::{SquadOvError, HalResponse, BlobManagementClient, JobQueue};
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
    pub connections: u32
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
pub struct ApiConfig {
    fusionauth: fusionauth::FusionAuthConfig,
    pub gcp: squadov_common::GCPConfig,
    pub database: DatabaseConfig,
    pub cors: CorsConfig,
    pub server: ServerConfig,
    pub gitlab: GitlabConfig
}

struct ApiClients {
    fusionauth: fusionauth::FusionAuthClient,
}

pub struct ApiApplication {
    pub config: ApiConfig,
    clients: ApiClients,
    users: auth::UserManager,
    session: auth::SessionManager,
    vod: Arc<dyn v1::VodManager + Send + Sync>,
    pool: Arc<PgPool>,
    schema: Arc<graphql::GraphqlSchema>,
    blob: Arc<BlobManagementClient>,
    // Various local job queues - these should eventually
    // probably be switched to something like RabbitMQ + microservices.
    pub vod_fastify_jobs: Arc<JobQueue<v1::VodFastifyJob>>
}

impl ApiApplication {
    pub async fn new(config: &ApiConfig) -> ApiApplication {
        // Use TOML config to create application - e.g. for
        // database configuration, external API client configuration, etc.
        let pool = Arc::new(PgPoolOptions::new()
            .max_connections(config.database.connections)
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
        ApiApplication{
            config: config.clone(),
            clients: ApiClients{
                fusionauth: fusionauth::FusionAuthClient::new(config.fusionauth.clone()),
            },
            users: auth::UserManager{},
            session: auth::SessionManager::new(),
            vod: match v1::get_current_vod_manager_type() {
                v1::VodManagerType::GCS => Arc::new(v1::GCSVodManager::new(gcp.clone()).await.unwrap()) as Arc<dyn v1::VodManager + Send + Sync>,
                v1::VodManagerType::FileSystem => Arc::new(v1::FilesystemVodManager::new().unwrap()) as Arc<dyn v1::VodManager + Send + Sync>
            },
            pool: pool,
            schema: Arc::new(graphql::create_schema()),
            blob: blob,
            // TODO: Configify!
            vod_fastify_jobs: JobQueue::new::<v1::VodFastifyWorker>(4),
        }
    }
}