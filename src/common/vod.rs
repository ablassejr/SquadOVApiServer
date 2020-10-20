use serde::{Serialize,Deserialize};
use uuid::Uuid;
use base64::decode_config;
use std::str;
use std::str::FromStr;
use std::clone::Clone;

#[derive(Serialize,Deserialize,Clone)]
pub struct VodMetadata {
    #[serde(rename = "videoUuid", default)]
    pub video_uuid: Uuid,
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
    pub fname: String
}

#[derive(Deserialize)]
pub struct VodSegmentId {
    pub video_uuid: Uuid,
    pub quality: String,
    pub segment_name: String
}

#[derive(Serialize,Deserialize)]
pub struct VodSegment {
    pub uri: String,
    pub duration: f32,
    #[serde(rename="segmentStart")]
    pub segment_start: f32
}

#[derive(Serialize,Deserialize)]
pub struct VodTrack {
    pub metadata: VodMetadata,
    pub segments: Vec<VodSegment>
}

#[derive(Serialize,Deserialize)]
pub struct VodManifest {
    #[serde(rename="videoTracks")]
    pub video_tracks: Vec<VodTrack>
}

impl Default for VodManifest {
    fn default() -> Self {
        return Self{
            video_tracks: Vec::new()
        }
    }
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