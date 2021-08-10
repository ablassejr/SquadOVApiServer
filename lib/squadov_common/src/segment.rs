use crate::SquadOvError;
use serde::{Deserialize, Serialize};
use reqwest::{StatusCode, header};

#[derive(Deserialize, Debug, Clone)]
pub struct SegmentConfig {
    write_key: String,
}

pub struct SegmentClient {
    config: SegmentConfig
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct ServerUserAddressTraits {
    pub city: Option<String>,
    pub country: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct ServerUserIdentifyTraits {
    pub email: String,
    pub username: String,
    pub address: Option<ServerUserAddressTraits>,
    pub city: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<i32>,
    pub cpu_vendor: Option<String>,
    pub cpu_brand: Option<String>,
    pub cpu_clock: Option<i64>,
    pub cpu_cores: Option<i32>,
    pub os_name: Option<String>,
    pub os_major: Option<String>,
    pub os_minor: Option<String>,
    pub os_edition: Option<String>,
    pub gpu_name_0: Option<String>,
    pub gpu_memory_0: Option<i64>,
    pub gpu_name_1: Option<String>,
    pub gpu_memory_1: Option<i64>,
    pub ram_kb: Option<i64>,
    pub squads: i64,
    pub squad_friends: i64,
    pub squad_invites_sent: i64,
    pub squad_link_used: i64,
    pub referrals: i64,
    pub clip_likes: i64,
    pub clip_comments: i64,
    pub aimlab_vods: i64,
    pub csgo_vods: i64,
    pub hearthstone_vods: i64,
    pub lol_vods: i64,
    pub tft_vods: i64,
    pub valorant_vods: i64,
    pub wow_vods: i64,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct SegmentContext {
    ip: String,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct IdentifyRequest {
    user_id: String,
    anonymous_id: String,
    context: SegmentContext,
    traits: ServerUserIdentifyTraits,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct TrackRequest {
    user_id: String,
    event: String,
}

impl SegmentClient {
    pub fn new(config: SegmentConfig) -> Self {
        Self {
            config,
        }
    }

    fn create_http_client(&self) -> Result<reqwest::Client, SquadOvError> {
        let mut headers = header::HeaderMap::new();
        let access_token = format!("Basic {}", base64::encode(&format!("{}:", &self.config.write_key)));
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&access_token)?);
        Ok(reqwest::ClientBuilder::new()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(60))
            .build()?)
    }

    pub async fn track(&self, user_id: &str, event: &str) -> Result<(), SquadOvError> {
        let req = TrackRequest{
            user_id: user_id.to_string(),
            event: event.to_string(),
        };
        
        let client = self.create_http_client()?;
        let resp = client.post("https://api.segment.io/v1/track")
            .json(&req)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::BadRequest);
        }

        Ok(())
    }

    pub async fn identify(&self, user_id: &str, anonymous_id: &str, ip: &str, traits: &ServerUserIdentifyTraits) -> Result<(), SquadOvError> {
        let req = IdentifyRequest{
            user_id: user_id.to_string(),
            anonymous_id: anonymous_id.to_string(),
            context: SegmentContext {
                ip: ip.to_string(),
            },
            traits: traits.clone(),
        };

        // Manually create the integrations object as a string because it's a pain in the ass to do in a type safe way.
        let integrations = format!(
            r#"{{
                "Google Analytics": {ga},
                "Vero": {vero},
                "Mixpanel": {mixpanel}
            }}"#,
            ga=if anonymous_id.is_empty() {
                String::from("false")
            } else {
                format!(r#"{{
                    "clientId": "{}"
                }}"#, anonymous_id)
            },
            vero="true",
            mixpanel=if anonymous_id.is_empty() {
                "false"
            } else {
                "true"
            },
        );

        let mut raw_value = serde_json::to_value(req)?;
        if let serde_json::Value::Object(m) = &mut raw_value {
            let value = serde_json::from_str(&integrations)?;
            m.insert(String::from("integrations"), value);
        }

        let client = self.create_http_client()?;
        let resp = client.post("https://api.segment.io/v1/identify")
            .json(&raw_value)
            .send()
            .await?;

        if resp.status() != StatusCode::OK {
            return Err(SquadOvError::BadRequest);
        }

        Ok(())
    }
}