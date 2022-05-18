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

pub struct WowEncounterReportGenerator<'a> {
    writer: Option<CombatLogAvroFileIO<'a>>,
    pending_encounters: HashMap<i32, DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowEncounterEventReport {
    pub encounter_name: String,
    #[serde(with = "ts_milliseconds")]
    pub start_tm: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub end_tm: DateTime<Utc>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_encounter_events",
        "fields": [
            {"name": "encounterName", "type": "string"},
            {"name": "startTm", "type": "long", "logicalType": "timestamp-millis"},
            {"name": "endTm", "type": "long", "logicalType": "timestamp-millis"}
        ]
    }
"#;

lazy_static! {
    static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
}

impl<'a> CombatLogReportHandler for WowEncounterReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    event: WoWCombatLogEventType::EncounterStart{encounter_id, ..},
                    ..
                }
            } => {
                self.pending_encounters.insert(*encounter_id, timestamp.clone());
            },
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    event: WoWCombatLogEventType::EncounterEnd{encounter_id, encounter_name, ..},
                    ..
                }
            } => {
                if let Some(start_tm) = self.pending_encounters.remove(&encounter_id) {
                    if let Some(w) = self.writer.as_mut() {
                        w.handle(WowEncounterEventReport{
                            encounter_name: encounter_name.clone(),
                            start_tm,
                            end_tm: timestamp.clone(),
                        })?;
                    }
                }
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowEncounterReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            writer: None,
            pending_encounters: HashMap::new(),
        }
    }
}

impl<'a> CombatLogReportIO for WowEncounterReportGenerator<'a> {
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
                        key_name: String::from("encounters.avro"),
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