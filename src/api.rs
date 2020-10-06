pub mod api_service;
pub mod auth;
pub mod fusionauth;

use serde::{Deserialize};
use std::fs;
use std::io;

#[derive(Deserialize,Debug)]
struct ApiConfig {
    fusionauth: fusionauth::FusionAuthConfig,
    session: auth::SessionConfig,
}

struct ApiClients {
    fusionauth: fusionauth::FusionAuthClient,
}

pub struct ApiApplication {
    clients: ApiClients,
    session: auth::SessionManager,
}

impl ApiApplication {
    pub fn new(config_path: std::path::PathBuf) -> io::Result<ApiApplication> {
        // Load TOML config.
        info!("Reading app config from: {:?}", config_path.to_str());
        let raw_cfg = fs::read_to_string(config_path)?;
        let config : ApiConfig = toml::from_str(&raw_cfg).unwrap();

        // Use TOML config to create application - e.g. for
        // database configuration, external API client configuration, etc.
        return Ok(ApiApplication{
            clients: ApiClients{
                fusionauth: fusionauth::FusionAuthClient::new(config.fusionauth),
            },
            session: auth::SessionManager::new(config.session),
        })
    }
}