mod create;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::Deserialize;
use crate::common;

#[derive(Deserialize)]
pub struct ValorantMatchMetadata {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "gameMode")]
    pub game_mode: String,
    #[serde(rename = "mapId")]
    pub map_id: String,
    #[serde(rename = "isRanked")]
    pub is_ranked: bool,
    #[serde(rename = "provisioningFlowID")]
    pub provisioning_flow_id: String,
    #[serde(rename = "gameVersion")]
    pub game_version: String,
    #[serde(rename = "gameStartMillis", deserialize_with="common::parse_utc_time_from_milliseconds")]
    pub server_start_time_utc: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct FullValorantMatchData {
    // Our internal UUID
    #[serde(default)]
    pub match_uuid: Uuid,
    #[serde(rename = "matchInfo")]
    pub match_info: ValorantMatchMetadata,
    #[serde(default)]
    pub raw_data: serde_json::Value
}

impl Default for ValorantMatchMetadata {
    fn default() -> ValorantMatchMetadata {
        ValorantMatchMetadata {
            match_id: String::default(),
            game_mode: String::default(),
            map_id: String::default(),
            is_ranked: false,
            provisioning_flow_id: String::default(),
            game_version: String::default(),
            server_start_time_utc: Utc::now(),
        }
    }
}

impl Default for FullValorantMatchData {
    fn default() -> FullValorantMatchData {
        FullValorantMatchData {
            match_info: ValorantMatchMetadata::default(),
            match_uuid: Uuid::nil(),
            raw_data: serde_json::Value::Null
        }
    }
}

pub use create::*;