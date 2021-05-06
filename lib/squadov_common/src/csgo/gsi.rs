use serde::Deserialize;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiWeaponState {
    pub name: String,
    pub r#type: String,
    pub paintkit: String,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiPlayerRoundState {
    pub steam_id: String,
    pub team: String,
    pub weapons: Vec<CsgoGsiWeaponState>,
    pub money: i32,
    pub equipment_value: i32,
    pub armor: i32,
    pub helmet: bool,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiKillState {
    pub timestamp: DateTime<Utc>,
    pub killer: Option<String>,
    pub victim: Option<String>,
    pub assisters: Vec<String>,
    pub weapon: Option<String>,
    pub headshot: Option<bool>,
    pub flashed: Option<bool>,
    pub smoked: Option<bool>,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiPlayerState {
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub mvps: i32,
    pub score: i32,
    pub name: String,
    pub steam_id: String,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiRoundState {
    pub round_num: i32,
    pub winning_team: Option<String>,
    pub round_win_method: Option<String>,
    pub players: HashMap<String, CsgoGsiPlayerRoundState>,
    pub kills: Vec<CsgoGsiKillState>,
    pub buy_time: Option<DateTime<Utc>>,
    pub play_time: Option<DateTime<Utc>>,
    pub bomb_plant_time: Option<DateTime<Utc>>,
    pub bomb_next_time: Option<DateTime<Utc>>,
    pub round_end_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiMatchState {
    pub map: String,
    pub mode: String,
    pub winner: Option<String>,
    pub connected_server: Option<String>,
    pub warmup_start: Option<DateTime<Utc>>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    pub rounds: Vec<CsgoGsiRoundState>,
    pub players: HashMap<String, CsgoGsiPlayerState>,
}