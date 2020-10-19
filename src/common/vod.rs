use serde::Deserialize;
use uuid::Uuid;
use base64::decode_config;
use std::str;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct VodMetadata {
    #[serde(rename = "resX")]
    pub res_x: i32,
    #[serde(rename = "resY")]
    pub res_y: i32,

    #[serde(rename = "minBitrate")]
    pub min_bitrate: i64,
    #[serde(rename = "avgBitrate")]
    pub avg_bitrate: i64,
    #[serde(rename = "maxBitrate")]
    pub max_bitrate: i64,

    pub id: String,
    #[serde(rename = "type")]
    pub data_type: String
}

pub struct VodStreamKey {
    pub user_uuid: Uuid,
    pub vod_uuid: Uuid
}

pub fn parse_vod_stream_key(key: &str) -> Result<VodStreamKey, super::SquadOvError> {
    let split: Vec<&str> = key.split('.').collect();

    if split.len() != 2 {
        return Err(super::SquadOvError::BadRequest);
    }

    let user_uuid = decode_config(split[0], base64::URL_SAFE_NO_PAD)?;
    let vod_uuid = decode_config(split[1], base64::URL_SAFE_NO_PAD)?;

    let user_uuid = str::from_utf8(&user_uuid)?;
    let vod_uuid = str::from_utf8(&vod_uuid)?;

    return Ok(VodStreamKey{
        user_uuid: Uuid::from_str(&user_uuid)?,
        vod_uuid: Uuid::from_str(&vod_uuid)?,
    })
}