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

pub struct WowAuraBreakReportGenerator<'a> {
    writer: Option<CombatLogAvroFileIO<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowAuraBreakEventReport {
    pub source_guid: String,
    pub source_name: String,
    pub source_flags: i64,
    pub target_guid: String,
    pub target_name: String,
    pub target_flags: i64,
    pub aura_id: i64,
    pub aura_type: WoWSpellAuraType,
    pub spell_id: Option<i64>,
    #[serde(with = "ts_milliseconds")]
    pub tm: DateTime<Utc>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_aura_break_events",
        "fields": [
            {"name": "sourceGuid", "type": "string"},
            {"name": "sourceName", "type": "string"},
            {"name": "sourceFlags", "type": "long"},
            {"name": "targetGuid", "type": "string"},
            {"name": "targetName", "type": "string"},
            {"name": "targetFlags", "type": "long"},
            {"name": "auraId", "type": "long"},
            {"name": "auraType", "type": {
                "type": "record",
                "name": "WowAuraType",
                "fields": [
                    {"name": "type", "type": "string"}
                ]
            }},
            {"name": "spellId", "type": ["null", "long"]},
            {"name": "tm", "type": "long", "logicalType": "timestamp-millis"}
        ]
    }
"#;

lazy_static! {
    static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
}


impl<'a> CombatLogReportHandler for WowAuraBreakReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    source: Some(src),
                    dest: Some(dst),
                    event: WoWCombatLogEventType::AuraBreak{aura, spell, aura_type},
                    ..
                }
            } => {
                if let Some(w) = self.writer.as_mut() {
                    w.handle(WowAuraBreakEventReport{
                        source_guid: src.guid.clone(),
                        source_name: src.name.clone(),
                        source_flags: src.flags,
                        target_guid: dst.guid.clone(),
                        target_name: dst.name.clone(),
                        target_flags: dst.flags,
                        aura_id: aura.id,
                        aura_type: aura_type.clone(),
                        spell_id: spell.as_ref().map(|x| { x.id }),
                        tm: timestamp.clone(),
                    })?;
                }
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowAuraBreakReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            writer: None,
        }
    }
}

impl<'a> CombatLogReportIO for WowAuraBreakReportGenerator<'a> {
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
                        key_name: String::from("aura_breaks.avro"),
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