use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportHandler,
        CombatLogReportIO,
        CombatLogReport,
        io::{
            avro::CombatLogAvroFileIO,
            CombatLogDiskIO,
        },
        RawStaticCombatLogReport,
    },
    wow::{
        reports::WowReportTypes,
        combatlog::{
            WowCombatLogPacket,
            WoWCombatLogEventType,
            WowPacketData,
            WoWCombatLogEvent,
            WoWSpellAuraType,
        },
    },
};
use std::sync::Arc;
use async_std::sync::{RwLock};
use serde::{Deserialize, Serialize};
use avro_rs::{
    Schema,
};
use chrono::{DateTime, Utc, serde::ts_milliseconds};
use std::collections::HashMap;

pub struct WowAuraReportGenerator<'a> {
    writer: Option<CombatLogAvroFileIO<'a>>,
    pending_auras: HashMap<(String, i64), DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowAuraEventReport {
    pub target_guid: String,
    pub target_name: String,
    pub spell_id: i64,
    pub aura_type: WoWSpellAuraType,
    #[serde(with = "ts_milliseconds")]
    pub applied_tm: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub removed_tm: DateTime<Utc>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_aura_events",
        "fields": [
            {"name": "targetGuid", "type": "string"},
            {"name": "targetName", "type": "string"},
            {"name": "spellId", "type": "long"},
            {"name": "auraType", "type": {
                "type": "record",
                "name": "WowAuraType",
                "fields": [
                    {"name": "type", "type": "string"}
                ]
            }},
            {"name": "appliedTm", "type": "long", "logicalType": "timestamp-millis"},
            {"name": "removedTm", "type": "long", "logicalType": "timestamp-millis"}
        ]
    }
"#;

lazy_static! {
    pub static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
}


impl<'a> CombatLogReportHandler for WowAuraReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    dest: Some(edest),
                    event: WoWCombatLogEventType::SpellAura{spell, aura_type, applied},
                    ..
                }
            } => {
                let key = (edest.guid.clone(), spell.id);
                if *applied {
                    self.pending_auras.insert(key, timestamp.clone());
                } else if let Some(start_tm) = self.pending_auras.remove(&key) {
                    if let Some(w) = self.writer.as_mut() {
                        w.handle(WowAuraEventReport{
                            target_guid: edest.guid.clone(),
                            target_name: edest.name.clone(),
                            spell_id: spell.id,
                            aura_type: aura_type.clone(),
                            applied_tm: start_tm,
                            removed_tm: timestamp.clone(),
                        })?;
                    }
                }                
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowAuraReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            writer: None,
            pending_auras: HashMap::new(),
        }
    }
}

impl<'a> CombatLogReportIO for WowAuraReportGenerator<'a> {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.writer = Some(
            CombatLogAvroFileIO::new(dir, &REPORT_SCHEMA)?
        );
        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        Ok(
            if let Some(w) = self.writer.take() {
                vec![
                    Arc::new(RawStaticCombatLogReport{
                        key_name: String::from("auras.avro"),
                        raw_file: RwLock::new(w.get_underlying_file()?),
                        canonical_type: WowReportTypes::Events as i32,
                    })
                ]
            } else {
                vec![]
            }
        )
    }
}