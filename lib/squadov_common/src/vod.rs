pub mod fastify;

use serde::{Serialize,Deserialize};
use uuid::Uuid;
use std::str;
use std::clone::Clone;

#[derive(Serialize,Deserialize,Clone)]
pub struct VodMetadata {
    #[serde(rename = "videoUuid", default)]
    pub video_uuid: Uuid,
    #[serde(rename = "resX")]
    pub res_x: i32,
    #[serde(rename = "resY")]
    pub res_y: i32,
    pub fps: i32,

    #[serde(rename = "minBitrate")]
    pub min_bitrate: i64,
    #[serde(rename = "avgBitrate")]
    pub avg_bitrate: i64,
    #[serde(rename = "maxBitrate")]
    pub max_bitrate: i64,

    pub id: String,
    #[serde(skip)]
    pub has_fastify: bool,
}

#[derive(Deserialize,Debug)]
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