mod backfill;
mod create;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::Deserialize;
use crate::common;
use std::collections::HashMap;

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
pub struct ValorantMatchTeamData {
    #[serde(rename = "teamId")]
    team_id: String,
    won: bool,
    #[serde(rename = "roundsPlayed")]
    rounds_played: i32,
    #[serde(rename = "roundsWon")]
    rounds_won: i32
}

#[derive(Deserialize)]
pub struct ValorantMatchPlayerStats {
    score: i32,
    #[serde(rename = "roundsPlayed")]
    rounds_played: i32,
    kills: i32,
    deaths: i32,
    assists: i32
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct ValorantMatchDamageData {
    receiver: String,
    damage: i32,
    legshots: i32,
    bodyshots: i32,
    headshots: i32
}

#[derive(Deserialize)]
pub struct ValorantMatchPlayerRoundStatsData {
    subject: String,
    score: i32,
    damage: Vec<ValorantMatchDamageData>
}

#[derive(Deserialize)]
pub struct ValorantMatchPlayerRoundEconomyData {
    subject: String,
    armor: String,
    weapon: String,
    remaining: i32,
    #[serde(rename = "loadoutValue")]
    loadout_value: i32,
    spent: i32
}

#[derive(Deserialize)]
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

type ValorantPerRoundPlayerDamageData<'a> = HashMap<String, &'a Vec<ValorantMatchDamageData>>;
type ValorantAllPlayerDamageData<'a> = HashMap<i32, ValorantPerRoundPlayerDamageData<'a>>;

#[derive(Deserialize)]
pub struct ValorantMatchKillFinishingDamage {
    #[serde(rename = "damageType")]
    damage_type: String,
    #[serde(rename = "damageItem")]
    damage_item: String,
    #[serde(rename = "isSecondaryFireMode")]
    is_secondary_fire_mode: bool
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
pub struct FullValorantMatchData {
    // Our internal UUID
    #[serde(default)]
    pub match_uuid: Uuid,
    #[serde(rename = "matchInfo")]
    pub match_info: ValorantMatchMetadata,
    pub teams: Vec<ValorantMatchTeamData>,
    pub players: Vec<ValorantMatchPlayerData>,
    #[serde(rename = "roundResults")]
    pub rounds: Vec<ValorantMatchRoundData>,
    pub kills: Vec<ValorantMatchKillData>,
    #[serde(default)]
    pub raw_data: serde_json::Value
}

#[derive(Deserialize)]
pub struct ValorantPlayerRoundMetadata {
    #[serde(rename = "matchId")]
    pub match_id: String,
    pub puuid: String,
    pub round: i32,
    #[serde(rename = "buyTime")]
    pub buy_time: DateTime<Utc>,
    #[serde(rename = "roundTime")]
    pub round_time: DateTime<Utc>
}

#[derive(Deserialize)]
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

pub use backfill::*;
pub use create::*;