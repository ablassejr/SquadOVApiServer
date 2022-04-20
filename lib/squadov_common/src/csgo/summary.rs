use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoPlayerMatchSummary {
    pub match_uuid: Uuid,
    pub user_uuid: Uuid,
    pub map: String,
    pub mode: String,
    pub match_start_time: DateTime<Utc>,
    pub match_length_seconds: i32,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub mvps: i32,
    pub winner: bool,
    pub has_demo: bool,
    pub headshots: i32,
    pub bodyshots: i32,
    pub legshots: i32,
    pub damage_per_round: f64,
    pub friendly_rounds: i32,
    pub enemy_rounds: i32,
    pub steam_id: i64,
}