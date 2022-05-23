use serde::Serialize;
use chrono::{DateTime, Utc};
use super::{
    WoWSpellAuraType
};
use crate::{
    wow::reports::events::{
        aura_breaks::WowAuraBreakEventReport,
        auras::WowAuraEventReport,
        deaths::WowDeathEventReport,
        encounters::WowEncounterEventReport,
        resurrections::WowResurrectionEventReport,
        spell_casts::WowSpellCastEventReport,
    }
};

#[derive(Serialize)]
pub struct SerializedWoWResurrection {
    pub guid: String,
    pub name: String,
    pub flags: i64,
    pub tm: DateTime<Utc>
}

impl From<WowResurrectionEventReport> for SerializedWoWResurrection {
    fn from(x: WowResurrectionEventReport) -> Self {
        Self {
            guid: x.guid,
            name: x.name,
            flags: x.flags,
            tm: x.tm,
        }
    }
}

impl From<SerializedWoWResurrection> for WowResurrectionEventReport {
    fn from(x: SerializedWoWResurrection) -> Self {
        Self {
            guid: x.guid,
            name: x.name,
            flags: x.flags,
            tm: x.tm,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct SerializedWoWDeath {
    pub event_id: i64,
    pub guid: String,
    pub name: String,
    pub flags: i64,
    pub tm: DateTime<Utc>
}

impl From<WowDeathEventReport> for SerializedWoWDeath {
    fn from(x: WowDeathEventReport) -> Self {
        Self {
            event_id: x.event_id,
            guid: x.guid,
            name: x.name,
            flags: x.flags,
            tm: x.tm,
        }
    }
}

impl From<SerializedWoWDeath> for WowDeathEventReport {
    fn from(x: SerializedWoWDeath) -> Self {
        Self {
            event_id: x.event_id,
            guid: x.guid,
            name: x.name,
            flags: x.flags,
            tm: x.tm,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct SerializedWoWAura {
    pub target_guid: String,
    pub target_name: String,
    pub spell_id: i64,
    pub aura_type: WoWSpellAuraType,
    pub applied_tm: DateTime<Utc>,
    pub removed_tm: DateTime<Utc>
}

impl From<WowAuraEventReport> for SerializedWoWAura {
    fn from(x: WowAuraEventReport) -> Self {
        Self {
            target_guid: x.target_guid,
            target_name: x.target_name,
            spell_id: x.spell_id,
            aura_type: x.aura_type,
            applied_tm: x.applied_tm,
            removed_tm: x.removed_tm,
        }
    }
}

impl From<SerializedWoWAura> for WowAuraEventReport {
    fn from(x: SerializedWoWAura) -> Self {
        Self {
            target_guid: x.target_guid,
            target_name: x.target_name,
            spell_id: x.spell_id,
            aura_type: x.aura_type,
            applied_tm: x.applied_tm,
            removed_tm: x.removed_tm,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct SerializedWowEncounter {
    pub encounter_name: String,
    pub start_tm: DateTime<Utc>,
    pub end_tm: DateTime<Utc>
}

impl From<WowEncounterEventReport> for SerializedWowEncounter {
    fn from(x: WowEncounterEventReport) -> Self {
        Self {
            encounter_name: x.encounter_name,
            start_tm: x.start_tm,
            end_tm: x.end_tm,
        }
    }
}

impl From<SerializedWowEncounter> for WowEncounterEventReport {
    fn from(x: SerializedWowEncounter) -> Self {
        Self {
            encounter_name: x.encounter_name,
            start_tm: x.start_tm,
            end_tm: x.end_tm,
        }
    }
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
    pub spell_id: i64,
    pub spell_school: i32,
    pub success: bool,
    pub instant: bool,
}

impl From<WowSpellCastEventReport> for SerializedWoWSpellCast {
    fn from(x: WowSpellCastEventReport) -> Self {
        Self {
            source_guid: x.source_guid,
            source_name: x.source_name,
            source_flags: x.source_flags,
            target_guid: x.target_guid,
            target_name: x.target_name,
            target_flags: x.target_flags,
            cast_start: x.cast_start,
            cast_finish: x.cast_finish,
            spell_id: x.spell_id,
            spell_school: x.spell_school,
            success: x.success,
            instant: x.instant,
        }
    }
}

impl From<SerializedWoWSpellCast> for WowSpellCastEventReport {
    fn from(x: SerializedWoWSpellCast) -> Self {
        Self {
            source_guid: x.source_guid,
            source_name: x.source_name,
            source_flags: x.source_flags,
            target_guid: x.target_guid,
            target_name: x.target_name,
            target_flags: x.target_flags,
            cast_start: x.cast_start,
            cast_finish: x.cast_finish,
            spell_id: x.spell_id,
            spell_school: x.spell_school,
            success: x.success,
            instant: x.instant,
        }
    }
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

impl From<WowAuraBreakEventReport> for SerializedWoWAuraBreak {
    fn from(x: WowAuraBreakEventReport) -> Self {
        Self {
            source_guid: x.source_guid,
            source_name: x.source_name,
            source_flags: x.source_flags,
            target_guid: x.target_guid,
            target_name: x.target_name,
            target_flags: x.target_flags,
            aura_id: x.aura_id,
            aura_type: x.aura_type,
            spell_id: x.spell_id,
            tm: x.tm,
        }
    }
}

impl From<SerializedWoWAuraBreak> for WowAuraBreakEventReport {
    fn from(x: SerializedWoWAuraBreak) -> Self {
        Self {
            source_guid: x.source_guid,
            source_name: x.source_name,
            source_flags: x.source_flags,
            target_guid: x.target_guid,
            target_name: x.target_name,
            target_flags: x.target_flags,
            aura_id: x.aura_id,
            aura_type: x.aura_type,
            spell_id: x.spell_id,
            tm: x.tm,
        }
    }
}