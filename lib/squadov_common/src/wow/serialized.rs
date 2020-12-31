use serde::Serialize;
use chrono::{DateTime, Utc};
use super::{
    WoWSpellAuraType
};

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
    #[serde(rename="spellName")]
    pub spell_name: String,
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