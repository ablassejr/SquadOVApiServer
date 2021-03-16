use serde::Serialize;
use chrono::{DateTime, Utc};
use super::{
    WoWSpellAuraType
};

#[derive(Serialize)]
pub struct SerializedWoWResurrection {
    pub guid: String,
    pub name: String,
    pub flags: i64,
    pub tm: DateTime<Utc>
}

#[derive(Serialize)]
pub struct SerializedWoWDeath {
    pub guid: String,
    pub name: String,
    pub flags: i64,
    pub tm: DateTime<Utc>
}

#[derive(Serialize)]
pub struct SerializedWoWAura {
    #[serde(rename="targetGuid")]
    pub target_guid: String,
    #[serde(rename="targetName")]
    pub target_name: String,
    #[serde(rename="spellId")]
    pub spell_id: i64,
    #[serde(rename="auraType")]
    pub aura_type: WoWSpellAuraType,
    #[serde(rename="appliedTm")]
    pub applied_tm: DateTime<Utc>,
    #[serde(rename="removedTm")]
    pub removed_tm: DateTime<Utc>
}

#[derive(Serialize)]
pub struct SerializedWowEncounter {
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    #[serde(rename="startTm")]
    pub start_tm: DateTime<Utc>,
    #[serde(rename="endTm")]
    pub end_tm: DateTime<Utc>
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct SerializedWoWSpellCast {
    pub source_guid: String,
    pub source_name: String,
    pub source_flags: i64,
    pub target_guid: Option<String>,
    pub target_name: Option<String>,
    pub target_flags: Option<i64>,
    pub cast_start: Option<DateTime<Utc>>,
    pub cast_finish: DateTime<Utc>,
    pub success: bool,
    pub instant: bool,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct SerializedWoWAuraBreak {
    pub source_guid: String,
    pub source_name: String,
    pub source_flags: i64,
    pub target_guid: String,
    pub target_name: String,
    pub target_flags: i64,
    pub aura_id: i64,
    pub aura_type: WoWSpellAuraType,
    pub spell_id: Option<i64>,
    pub tm: DateTime<Utc>,
}