use serde::Deserialize;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiWeaponState {
    name: String,
    r#type: String,
    paintkit: String,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiPlayerRoundState {
    steam_id: String,
    team: String,
    weapons: HashMap<i32, CsgoGsiWeaponState>,
    money: i32,
    equipment_value: i32,
    armor: i32,
    helmet: bool,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiKillState {
    timestamp: DateTime<Utc>,
    killer: Option<String>,
    victim: Option<String>,
    assisters: Vec<String>,
    weapon: Option<String>,
    headshot: Option<bool>,
    flashed: Option<bool>,
    smoked: Option<bool>,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiPlayerState {
    round_num: i32,
    winning_team: Option<String>,
    round_win_method: Option<String>,
    players: HashMap<String, CsgoGsiPlayerRoundState>,
    kills: Vec<CsgoGsiKillState>,
    buy_time: Option<DateTime<Utc>>,
    play_time: Option<DateTime<Utc>>,
    bomb_plant_time: Option<DateTime<Utc>>,
    bomb_next_time: Option<DateTime<Utc>>,
    round_end_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiRoundState {
    kills: i32,
    deaths: i32,
    assists: i32,
    mvps: i32,
    score: i32,
    name: String,
    steam_id: String,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoGsiMatchState {
    map: String,
    mode: String,
    winner: Option<String>,
    connected_server: Option<String>,
    warmup_start: Option<DateTime<Utc>>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    rounds: HashMap<i32, CsgoGsiRoundState>,
    players: HashMap<String, CsgoGsiPlayerState>,
}