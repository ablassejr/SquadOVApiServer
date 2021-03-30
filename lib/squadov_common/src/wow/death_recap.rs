use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecapEvent {
    pub tm: DateTime<Utc>,
    pub diff_ms: i64,
    pub diff_hp: i32,
    pub spell_id: Option<i64>,
    pub source_guid: Option<String>,
    pub source_name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecap {
    pub hp_events: Vec<WowDeathRecapEvent>
}