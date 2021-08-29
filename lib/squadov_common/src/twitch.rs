pub mod oauth;
pub mod api;
pub mod rabbitmq;
pub mod eventsub;

use serde::Deserialize;

#[derive(Deserialize,Debug,Clone)]
pub struct TwitchConfig {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub eventsub_hostname: String,
}