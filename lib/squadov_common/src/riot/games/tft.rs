use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

pub struct TftMatchLink {
    pub match_uuid: Uuid,
    pub platform: String,
    pub region: String,
    pub match_id: i64
}

#[derive(Deserialize)]
pub struct TftMatchDto {
    pub info: TftInfoDto,
}

#[derive(Serialize, Deserialize)]
pub struct TftInfoDto {
    #[serde(rename(serialize="gameDatetime"), deserialize_with="crate::parse_utc_time_from_seconds")]
    pub game_datetime: Option<DateTime<Utc>>,
    #[serde(rename(serialize="gameLength"))]
    pub game_length: f32,
    #[serde(rename(serialize="gameVariation"))]
    pub game_variation: Option<String>,
    #[serde(rename(serialize="gameVersion"))]
    pub game_version: String,
    #[serde(rename(serialize="queueId"))]
    pub queue_id: i32,
    #[serde(rename(serialize="tftSetNumber"))]
    pub tft_set_number: i32,
    pub participants: Vec<TftParticipantDto>,
}

#[derive(Serialize, Deserialize)]
pub struct TftParticipantDto {
    #[serde(rename(serialize="goldLeft"))]
    pub gold_left: i32,
    #[serde(rename(serialize="lastRound"))]
    pub last_round: i32,
    pub level: i32,
    pub placement: i32,
    #[serde(rename(serialize="playersEliminated"))]
    pub players_eliminated: i32,
    pub puuid: String,
    #[serde(rename(serialize="timeEliminated"))]
    pub time_eliminated: f32, // seconds
    #[serde(rename(serialize="totalDamageToPlayers"))]
    pub total_damage_to_players: i32,
    pub traits: Vec<TftTraitDto>,
    pub units: Vec<TftUnitDto>,
    pub companion: TftCompanionDto,
}

#[derive(Serialize,Deserialize)]
pub struct TftCompanionDto {
    #[serde(rename(deserialize="content_ID", serialize="contentId"))]
    pub content_id: String,
    #[serde(rename(deserialize="skin_ID", serialize="skinId"))]
    pub skin_id: i32,
    pub species: String,
}

#[derive(Serialize,Deserialize,Clone)]
pub struct TftTraitDto {
    pub name: String,
    #[serde(rename(serialize="numUnits"))]
    pub num_units: i32,
    pub style: i32,
    #[serde(rename(serialize="tierCurrent"))]
    pub tier_current: i32,
    #[serde(rename(serialize="tierTotal"))]
    pub tier_total: i32
}

#[derive(Serialize,Deserialize,Clone)]
pub struct TftUnitDto {
    pub items: Vec<i32>,
    #[serde(rename(serialize="characterId"))]
    pub character_id: Option<String>,
    pub chosen: Option<String>,
    pub name: String,
    pub rarity: i32,
    pub tier: i32
}