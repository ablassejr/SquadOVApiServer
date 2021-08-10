use crate::SquadOvError;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use chrono::{DateTime, Utc};
use reqwest::{StatusCode};

#[derive(Serialize, Debug, Clone)]
pub struct LocationData {
    pub city: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<i32>,
    pub cache_tm: DateTime<Utc>,
}

const IPSTACK_CACHE_VALID_LOCATION: i64 = 2628000;
const IPSTACK_CACHE_INVALID_LOCATION: i64 = 604800;

impl LocationData {
    pub fn expired(&self) -> bool {
        let seconds_since_cache = Utc::now().signed_duration_since(self.cache_tm).num_seconds();
        if self.city.is_none() || self.country.is_none() || self.timezone.is_none() {
            seconds_since_cache > IPSTACK_CACHE_INVALID_LOCATION
        } else {
            seconds_since_cache > IPSTACK_CACHE_VALID_LOCATION
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct IpstackConfig {
    api_key: String,
}

pub struct IpstackClient {
    config: IpstackConfig
}

impl IpstackClient {
    pub fn new(config: IpstackConfig) -> Self {
        Self {
            config,
        }
    }

    fn build_url(&self, ip: &str) -> String {
        format!(
            "https://api.ipstack.com/{}?access_key={}",
            ip,
            &self.config.api_key,
        )
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        Ok(reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .build()?)
    }

    pub async fn get_location_data(&self, ip: &IpAddr) -> Result<LocationData, SquadOvError> {
        let client = self.create_http_client()?;
        let resp = client.get(
            &self.build_url(&ip.to_string())
        )
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::NotFound);
        }

        #[derive(Deserialize)]
        struct ResponseTz {
            gmt_offset: i32,
        }

        #[derive(Deserialize)]
        struct Response {
            city: String,
            country_name: String,
            time_zone: ResponseTz,
        }

        let data = resp.json::<Response>().await?;
        Ok(LocationData{
            city: Some(data.city),
            country: Some(data.country_name),
            timezone: Some(data.time_zone.gmt_offset / 60 / 60),
            cache_tm: Utc::now(),
        })
    }
}