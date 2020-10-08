pub mod api_service;
pub mod auth;
pub mod fusionauth;
pub mod access;
pub mod v1;

use serde::{Deserialize};
use std::fs;
use std::io;
use sqlx::postgres::{PgPool, PgPoolOptions};

#[derive(Deserialize,Debug)]
struct DatabaseConfig {
    url: String,
    connections: u32
}

#[derive(Deserialize,Debug)]
pub struct CorsConfig {
    pub domain: String
}

#[derive(Deserialize,Debug)]
struct ApiConfig {
    fusionauth: fusionauth::FusionAuthConfig,
    database: DatabaseConfig,
    cors: CorsConfig
}

struct ApiClients {
    fusionauth: fusionauth::FusionAuthClient,
}

pub struct ApiApplication {
    pub cors: CorsConfig,
    clients: ApiClients,
    users: auth::UserManager,
    session: auth::SessionManager,
    pool: PgPool
}

impl ApiApplication {
    pub async fn new(config_path: std::path::PathBuf) -> io::Result<ApiApplication> {
        // Load TOML config.
        info!("Reading app config from: {:?}", config_path.to_str());
        let raw_cfg = fs::read_to_string(config_path)?;
        let config : ApiConfig = toml::from_str(&raw_cfg).unwrap();

        let pool = PgPoolOptions::new()
            .max_connections(config.database.connections)
            .connect(&config.database.url)
            .await
            .unwrap();

        // Use TOML config to create application - e.g. for
        // database configuration, external API client configuration, etc.
        return Ok(ApiApplication{
            cors: config.cors,
            clients: ApiClients{
                fusionauth: fusionauth::FusionAuthClient::new(config.fusionauth),
            },
            users: auth::UserManager{},
            session: auth::SessionManager::new(),
            pool: pool,
        })
    }
}