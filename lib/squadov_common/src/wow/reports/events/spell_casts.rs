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
use serde::Serialize;
use avro_rs::{
    Schema,
};
use chrono::{
    DateTime,
    Utc,
    serde::{
        ts_milliseconds,
        ts_milliseconds_option,
    }
};
use std::collections::HashMap;


#[derive(Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowSpellCastEventReport {
    source_guid: String,
    source_name: String,
    source_flags: i64,
    target_guid: Option<String>,
    target_name: Option<String>,
    target_flags: Option<i64>,
    #[serde(with = "ts_milliseconds_option")]
    cast_start: Option<DateTime<Utc>>,
    #[serde(with = "ts_milliseconds")]
    cast_finish: DateTime<Utc>,
    spell_id: i64,
    spell_school: i32,
    success: bool,
    instant: bool,
}

pub struct WowSpellCastReportGenerator<'a> {
    writer: Option<CombatLogAvroFileIO<'a>>,
    pending_spells: HashMap<(String, i64), WowSpellCastEventReport>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_spell_cast_events",
        "fields": [
            {"name": "sourceGuid", "type": "string"},
            {"name": "sourceName", "type": "string"},
            {"name": "sourceFlags", "type": "long"},
            {"name": "targetGuid", "type": ["null", "string"]},
            {"name": "targetName", "type": ["null", "string"]},
            {"name": "targetFlags", "type": ["null", "long"]},
            {"name": "castStart", "type": ["null", { "type": "long", "logicalType": "timestamp-millis"}]},
            {"name": "castFinish", "type": "long", "logicalType": "timestamp-millis"},
            {"name": "spellId", "type": "long"},
            {"name": "spellSchool", "type": "int"},
            {"name": "success", "type": "boolean"},
            {"name": "instant", "type": "boolean"}
        ]
    }
"#;

lazy_static! {
    static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
}


impl<'a> CombatLogReportHandler for WowSpellCastReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    dest,
                    source: Some(src),
                    event: WoWCombatLogEventType::SpellCast{spell, start, finish, success},
                    ..
                }
            } => {
                // There's a couple of situations that can happen here:
                //  1) Instant cast - Success
                //  2) Instant cast - Failure
                //  3) Spell with Cast Time
                //  4) Spell with Cast Time that never results in success or failure.
                // 
                // Detecting the first two is simple: when a spell "finish" comes in and there's no "start", we can assume that's an instant cast.
                // The 3rd case is also fairly trivial, we'll get a spell start and then a corresponding spell finish later on.
                // The tricky part is that the 4th situation exists where sometimes a spell starts casting but never finishes (success or failure). The only way
                // to really detect this situation is when the same spell gets cast by the same player again before the previous cast finishes.
                let key = (src.guid.clone(), spell.id);

                let mut start_report = WowSpellCastEventReport{
                    source_guid: src.guid.clone(),
                    source_name: src.name.clone(),
                    source_flags: src.flags,
                    target_guid: None,
                    target_name: None,
                    target_flags: None,
                    cast_start: Some(timestamp.clone()),
                    cast_finish: timestamp.clone(),
                    spell_id: spell.id,
                    spell_school: spell.school as i32,
                    success: false,
                    instant: false,
                };

                if *start {
                    // This is the detection of the spell with cast time that never results in a success or failure.
                    // Insert returns the old value if present. Thus if the old value is present, we detected an implicit
                    // cast failure. We've also prepped ourselves for detecting the full spell cast by inserting it into the 
                    // pending spells hashmap.
                    if let Some(previous) = self.pending_spells.insert(key.clone(), start_report) {
                        self.write_event(previous)?;
                    }
                } else if *finish {
                    // If there's a spell in the pending_spells hashmap, it's a spell cast with a cast time. Otherwise it's an instant cast.
                    // I'd imagine there's a way to condense the two branches.
                    if let Some(mut previous) = self.pending_spells.remove(&key) {
                        previous.success = *success;
                        previous.cast_finish = timestamp.clone();

                        if let Some(dst) = dest {
                            previous.target_guid = Some(dst.guid.clone());
                            previous.target_name = Some(dst.name.clone());
                            previous.target_flags = Some(dst.flags);
                        }

                        self.write_event(previous)?;
                    } else {
                        start_report.success = *success;
                        start_report.cast_finish = timestamp.clone();

                        if let Some(dst) = dest {
                            start_report.target_guid = Some(dst.guid.clone());
                            start_report.target_name = Some(dst.name.clone());
                            start_report.target_flags = Some(dst.flags);
                        }
                        
                        self.write_event(start_report)?;
                    }
                } else {
                    log::warn!("Detected a spell cast that's neither a start nor a finish?");
                }
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowSpellCastReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            writer: None,
            pending_spells: HashMap::new(),
        }
    }

    fn write_event(&mut self, data: WowSpellCastEventReport) -> Result<(), SquadOvError> {
        if let Some(w) = self.writer.as_mut() {
            w.handle(data)?;
        }
        Ok(())
    }
}

impl<'a> CombatLogReportIO for WowSpellCastReportGenerator<'a> {
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
                        key_name: String::from("spell_casts.avro"),
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