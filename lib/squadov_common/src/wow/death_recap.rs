use serde::Serialize;
use chrono::{DateTime, Utc};
use crate::wow::reports::events::deaths::WowDeathRecapHpEvent;

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

impl From<WowDeathRecapHpEvent> for WowDeathRecapEvent {
    fn from(x: WowDeathRecapHpEvent) -> Self {
        Self {
            tm: x.tm,
            diff_ms: x.diff_ms,
            diff_hp: x.diff_hp,
            spell_id: x.spell_id,
            source_guid: x.source_guid,
            source_name: x.source_name,
        }
    }
}


#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecap {
    pub hp_events: Vec<WowDeathRecapEvent>
}