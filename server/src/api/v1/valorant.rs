mod backfill;
mod create;
mod list;
mod stats;
mod get;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize,Deserialize)]
pub struct ValorantPlayerRoundMetadata {
    #[serde(rename = "matchUuid")]
    pub match_uuid: Uuid,
    pub puuid: String,
    pub round: i32,
    #[serde(rename = "buyTime")]
    pub buy_time: Option<DateTime<Utc>>,
    #[serde(rename = "roundTime")]
    pub round_time: Option<DateTime<Utc>>
}

#[derive(Serialize,Deserialize)]
pub struct ValorantPlayerMatchMetadata {
    #[serde(rename = "matchUuid")]
    pub match_uuid: Uuid,
    pub puuid: String,
    #[serde(rename = "startTime")]
    pub start_time: DateTime<Utc>,
    #[serde(rename = "endTime")]
    pub end_time: DateTime<Utc>,
    pub rounds: Vec<ValorantPlayerRoundMetadata>
}

#[derive(Serialize)]
pub struct ValorantPlayerStatsSummary {
    rank: i32,
    kills: i64,
    deaths: i64,
    assists: i64,
    rounds: i64,
    #[serde(rename = "totalCombatScore")]
    total_combat_score: i64,
    #[serde(rename = "totalDamage")]
    total_damage: i64,
    headshots: i64,
    bodyshots: i64,
    legshots: i64,
    wins: i64,
    games: i64
}

impl Default for ValorantPlayerStatsSummary {
    fn default() -> Self {
        Self {
            rank: 0,
            kills: 0,
            deaths: 0,
            assists: 0,
            rounds: 0,
            total_combat_score: 0,
            total_damage: 0,
            headshots: 0,
            bodyshots: 0,
            legshots: 0,
            wins: 0,
            games: 0
        }
    }
}

pub use backfill::*;
pub use create::*;
pub use list::*;
pub use stats::*;
pub use get::*;