use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecapEvent {
    pub tm: DateTime<Utc>,
    pub diff_ms: i64,
    pub diff_hp: i32,
    pub spell_id: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecap {
    pub hp_events: Vec<WowDeathRecapEvent>
}