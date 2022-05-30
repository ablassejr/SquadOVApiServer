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

pub struct WowResurrectionReportGenerator<'a> {
    writer: Option<CombatLogAvroFileIO<'a>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowResurrectionEventReport {
    pub guid: String,
    pub name: String,
    pub flags: i64,
    #[serde(with = "ts_milliseconds")]
    pub tm: DateTime<Utc>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_resurrection_events",
        "fields": [
            {"name": "guid", "type": "string"},
            {"name": "name", "type": "string"},
            {"name": "flags", "type": "long"},
            {"name": "tm", "type": "long", "logicalType": "timestamp-millis"}
        ]
    }
"#;

lazy_static! {
    pub static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
}

impl<'a> CombatLogReportHandler for WowResurrectionReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    dest: Some(edest),
                    event: WoWCombatLogEventType::Resurrect(..),
                    ..
                }
            } => {
                if let Some(w) = self.writer.as_mut() {
                    w.handle(WowResurrectionEventReport{
                        guid: edest.guid.clone(),
                        name: edest.name.clone(),
                        flags: edest.flags,
                        tm: timestamp.clone(),
                    })?;
                }
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowResurrectionReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            writer: None,
        }
    }
}

impl<'a> CombatLogReportIO for WowResurrectionReportGenerator<'a> {
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
                        key_name: String::from("resurrections.avro"),
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