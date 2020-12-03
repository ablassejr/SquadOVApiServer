mod backfill;
mod create;
mod list;
mod stats;
mod get;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use squadov_common;
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchMetadata {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "gameMode")]
    pub game_mode: Option<String>,
    #[serde(rename = "mapId")]
    pub map_id: Option<String>,
    #[serde(rename = "isRanked")]
    pub is_ranked: Option<bool>,
    #[serde(rename = "provisioningFlowID")]
    pub provisioning_flow_id: Option<String>,
    #[serde(rename = "gameVersion")]
    pub game_version: Option<String>,
    #[serde(rename(serialize="serverStartTimeUtc", deserialize="gameStartMillis"), deserialize_with="squadov_common::parse_utc_time_from_milliseconds")]
    pub server_start_time_utc: Option<DateTime<Utc>>,
}

impl Default for ValorantMatchMetadata {
    fn default() -> Self {
        Self {
            match_id: String::new(),
            game_mode: None,
            map_id: None,
            is_ranked: None,
            provisioning_flow_id: None,
            game_version: None,
            server_start_time_utc: None
        }
    }
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchTeamData {
    #[serde(rename = "teamId")]
    team_id: String,
    won: bool,
    #[serde(rename = "roundsPlayed")]
    rounds_played: i32,
    #[serde(rename = "roundsWon")]
    rounds_won: i32
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchPlayerStats {
    score: i32,
    #[serde(rename = "roundsPlayed")]
    rounds_played: i32,
    kills: i32,
    deaths: i32,
    assists: i32
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchPlayerData {
    subject: String,
    #[serde(rename = "characterId")]
    character_id: String,
    #[serde(rename = "competitiveTier", default)]
    competitive_tier: i32,
    #[serde(rename = "teamId")]
    team_id: String,
    stats: ValorantMatchPlayerStats
}

#[derive(Serialize,Deserialize,Clone)]
pub struct ValorantMatchDamageData {
    receiver: String,
    damage: i32,
    legshots: i32,
    bodyshots: i32,
    headshots: i32
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchPlayerRoundStatsData {
    subject: String,
    score: i32,
    damage: Vec<ValorantMatchDamageData>
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchPlayerRoundEconomyData {
    subject: String,
    armor: String,
    weapon: String,
    remaining: i32,
    #[serde(rename = "loadoutValue")]
    loadout_value: i32,
    spent: i32
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchRoundData {
    #[serde(rename = "roundNum")]
    round_num: i32,
    #[serde(rename = "plantRoundTime")]
    plant_round_time: Option<i32>,
    #[serde(rename = "bombPlanter")]
    bomb_planter: Option<String>,
    #[serde(rename = "defuseRoundTime")]
    defuse_round_time: Option<i32>,
    #[serde(rename = "bombDefuser")]
    bomb_defuser: Option<String>,
    #[serde(rename = "winningTeam")]
    winning_team: String,
    #[serde(rename = "playerStats")]
    player_stats: Vec<ValorantMatchPlayerRoundStatsData>,
    #[serde(rename = "playerEconomies")]
    player_economies: Option<Vec<ValorantMatchPlayerRoundEconomyData>>
}

type ValorantAllPlayerRoundStatsData<'a> = HashMap<i32, &'a Vec<ValorantMatchPlayerRoundStatsData>>;
type ValorantAllPlayerRoundEconomyData<'a> = HashMap<i32, &'a Vec<ValorantMatchPlayerRoundEconomyData>>;

// Do not change this from a BTreeMap to a HashMap. Insertion into the database RELIES on this being ordered for the uniqueness check!!!!!!
type ValorantPerRoundPlayerDamageData<'a> = BTreeMap<String, &'a Vec<ValorantMatchDamageData>>;
type ValorantAllPlayerDamageData<'a> = BTreeMap<i32, ValorantPerRoundPlayerDamageData<'a>>;

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchKillFinishingDamage {
    #[serde(rename = "damageType")]
    damage_type: String,
    #[serde(rename = "damageItem")]
    damage_item: String,
    #[serde(rename = "isSecondaryFireMode")]
    is_secondary_fire_mode: bool
}

#[derive(Serialize,Deserialize)]
pub struct ValorantMatchKillData {
    #[serde(rename = "roundTime")]
    round_time: i32,
    #[serde(rename = "round")]
    round: i32,
    #[serde(rename = "finishingDamage")]
    finishing_damage: ValorantMatchKillFinishingDamage,
    killer: Option<String>,
    victim: String
}

#[derive(Serialize,Deserialize)]
pub struct FullValorantMatchData {
    // Our internal UUID
    #[serde(rename = "matchUuid", default)]
    pub match_uuid: Uuid,
    #[serde(rename = "matchInfo")]
    pub match_info: ValorantMatchMetadata,
    pub teams: Vec<ValorantMatchTeamData>,
    pub players: Vec<ValorantMatchPlayerData>,
    #[serde(rename = "roundResults")]
    pub rounds: Vec<ValorantMatchRoundData>,
    pub kills: Vec<ValorantMatchKillData>,
    #[serde(rename = "rawData", default)]
    pub raw_data: serde_json::Value
}

impl Default for FullValorantMatchData {
    fn default() -> Self {
        Self {
            match_uuid: Uuid::nil(),
            match_info: ValorantMatchMetadata {
                ..Default::default()
            },
            teams: Vec::new(),
            players: Vec::new(),
            rounds: Vec::new(),
            kills: Vec::new(),
            raw_data: serde_json::Value::Null,
        }
    }
}

#[derive(Serialize,Deserialize)]
pub struct ValorantPlayerRoundMetadata {
    #[serde(rename = "matchId")]
    pub match_id: String,
    pub puuid: String,
    pub round: i32,
    #[serde(rename = "buyTime")]
    pub buy_time: Option<DateTime<Utc>>,
    #[serde(rename = "roundTime")]
    pub round_time: Option<DateTime<Utc>>
}

#[derive(Serialize,Deserialize)]
pub struct ValorantPlayerMatchMetadata {
    #[serde(rename = "matchId")]
    pub match_id: String,
    pub puuid: String,
    #[serde(rename = "startTime")]
    pub start_time: DateTime<Utc>,
    #[serde(rename = "endTime")]
    pub end_time: DateTime<Utc>,
    pub rounds: Vec<ValorantPlayerRoundMetadata>
}

#[derive(Serialize,Deserialize)]
pub struct ValorantPlayerMatchSummary {
    #[serde(rename = "matchId")]
    match_id: String,
    #[serde(rename = "matchUuid")]
    match_uuid: Uuid,
    #[serde(rename = "serverStartTimeUtc")]
    server_start_time_utc: Option<DateTime<Utc>>,
    #[serde(rename = "gameMode")]
    game_mode: Option<String>,
    map: Option<String>,
    #[serde(rename = "isRanked")]
    is_ranked: Option<bool>,
    #[serde(rename = "provisioningFlowID")]
    provisioning_flow_id: Option<String>,
    #[serde(rename = "gameVersion")]
    game_version: Option<String>,
    #[serde(rename = "characterId")]
    character_id: String,
    won: bool,
    #[serde(rename = "roundsWon")]
    rounds_won: i32,
    #[serde(rename = "roundsLost")]
    rounds_lost: i32,
    #[serde(rename = "combatScoreRank")]
    combat_score_rank: i64,
    #[serde(rename = "competitiveTier")]
    competitive_tier: i32,
    kills: i32,
    deaths: i32,
    assists: i32,
    #[serde(rename = "roundsPlayed")]
    rounds_played: i32,
    #[serde(rename = "totalCombatScore")]
    total_combat_score: i32,
    #[serde(rename = "totalDamage")]
    total_damage: i64,
    headshots: i64,
    bodyshots: i64,
    legshots: i64
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