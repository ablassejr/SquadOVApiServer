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
            WoWDamageType,
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
use std::collections::{HashMap, VecDeque};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecapHpEvent {
    #[serde(with = "ts_milliseconds")]
    tm: DateTime<Utc>,
    diff_ms: i64,
    diff_hp: i32,
    spell_id: Option<i64>,
    source_guid: Option<String>,
    source_name: Option<String>,
}

impl Default for WowDeathRecapHpEvent {
    fn default() -> Self {
        Self {
            tm: Utc::now(),
            diff_ms: 0,
            diff_hp: 0,
            spell_id: None,
            source_guid: None,
            source_name: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct WowDeathRecap {
    pub hp_events: Vec<WowDeathRecapHpEvent>,
}

const DEATH_RECAP_SECONDS: i64 = 5;

pub struct WowDeathEventsReportGenerator<'a> {
    work_dir: Option<String>,
    writer: Option<CombatLogAvroFileIO<'a>>,
    event_counter: i64,
    // We need to track all the HP changing events for every *player* so that we can pull that information when they die for the death recap
    hp_change_events: HashMap<String, VecDeque<WowDeathRecapHpEvent>>,
    completed_death_recaps: Vec<Arc<RawStaticCombatLogReport>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowDeathEventReport {
    pub event_id: i64,
    pub guid: String,
    pub name: String,
    pub flags: i64,
    #[serde(with = "ts_milliseconds")]
    pub tm: DateTime<Utc>,
}

const REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_death_events",
        "fields": [
            {"name": "eventId", "type": "long"},
            {"name": "guid", "type": "string"},
            {"name": "name", "type": "string"},
            {"name": "flags", "type": "long"},
            {"name": "tm", "type": "long", "logicalType": "timestamp-millis"}
        ]
    }
"#;

const DEATH_RECAP_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_death_report_events",
        "fields": [
            {"name": "tm", "type": "long", "logicalType": "timestamp-millis"},
            {"name": "diffMs", "type": "long"},
            {"name": "diffHp", "type": "int"},
            {"name": "spellId", "type": ["null", "long"]},
            {"name": "sourceGuid", "type": ["null", "string"]},
            {"name": "sourceName", "type": ["null", "string"]}
        ]
    }
"#;

lazy_static! {
    static ref REPORT_SCHEMA: Schema = Schema::parse_str(REPORT_SCHEMA_RAW).unwrap();
    static ref DEATH_RECAP_SCHEMA: Schema = Schema::parse_str(DEATH_RECAP_SCHEMA_RAW).unwrap();
}

impl<'a> CombatLogReportHandler for WowDeathEventsReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    dest: Some(edest),
                    event: WoWCombatLogEventType::UnitDied{unconcious: false},
                    ..
                }
            } => {
                if let Some(w) = self.writer.as_mut() {
                    w.handle(WowDeathEventReport{
                        event_id: self.event_counter,
                        guid: edest.guid.clone(),
                        name: edest.name.clone(),
                        flags: edest.flags,
                        tm: timestamp.clone(),
                    })?;
                    self.construct_death_report(self.event_counter, edest.guid.as_str(), timestamp.clone())?;
                    self.event_counter += 1;
                }
            },
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    source,
                    dest: Some(dst),
                    event: WoWCombatLogEventType::DamageDone{damage, amount, ..},
                    ..
                }
            } => {
                let mut event = WowDeathRecapHpEvent{
                    tm: timestamp.clone(),
                    diff_hp: -*amount as i32,
                    ..WowDeathRecapHpEvent::default()
                };

                if let Some(src) = source {
                    event.source_guid = Some(src.guid.clone());
                    event.source_name = Some(src.name.clone());
                }

                match damage {
                    WoWDamageType::SpellDamage(spell) => {
                        event.spell_id = Some(spell.id);
                    },
                    _ => (),
                }

                self.add_death_recap_hp_event(dst.guid.as_str(), event)?;
            },
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    source,
                    dest: Some(dst),
                    event: WoWCombatLogEventType::Healing{spell, amount, overheal, ..},
                    ..
                }
            } => {
                let mut event = WowDeathRecapHpEvent{
                    tm: timestamp.clone(),
                    diff_hp: std::cmp::max(*amount - *overheal, 0) as i32,
                    ..WowDeathRecapHpEvent::default()
                };

                if let Some(src) = source {
                    event.source_guid = Some(src.guid.clone());
                    event.source_name = Some(src.name.clone());
                }

                event.spell_id = Some(spell.id);
                self.add_death_recap_hp_event(dst.guid.as_str(), event)?;
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowDeathEventsReportGenerator<'a> {
    pub fn new() -> Self {
        Self {
            work_dir: None,
            writer: None,
            event_counter: 0,
            hp_change_events: HashMap::new(),
            completed_death_recaps: vec![],
        }
    }

    fn construct_death_report(&mut self, event_id: i64, guid: &str, ref_time: DateTime<Utc>) -> Result<(), SquadOvError> {
        // We don't want to potentially store a shit ton of events in memory so we flush it all to disk.
        if let Some(dir) = self.work_dir.as_ref() {
            let mut writer = CombatLogAvroFileIO::new(dir.as_str(), &DEATH_RECAP_SCHEMA)?;
            let events = self.hp_change_events.remove(guid).unwrap_or(VecDeque::new());
            
            for mut e in events {
                e.diff_ms = (e.tm - ref_time).num_milliseconds();
                writer.handle(e)?;
            }

            self.completed_death_recaps.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: format!("{}.avro", event_id),
                    raw_file: RwLock::new(writer.get_underlying_file()?),
                    canonical_type: WowReportTypes::DeathRecap as i32,
                })
            );
        }
        Ok(())
    }

    fn add_death_recap_hp_event(&mut self, guid: &str, event: WowDeathRecapHpEvent) -> Result<(), SquadOvError> {
        if let Some(all_events) = self.hp_change_events.get_mut(guid) {
            all_events.push_back(event);
        } else {
            self.hp_change_events.insert(guid.to_string(), VecDeque::from([event]));
        }

        // Now we need to make sure we never store more than DEATH_RECAP_SECONDS seconds worth of events.
        if let Some(all_events) = self.hp_change_events.get_mut(guid) {
            while !all_events.is_empty() && (all_events.back().unwrap().tm - all_events.front().unwrap().tm).num_seconds() > DEATH_RECAP_SECONDS {
                all_events.pop_front();
            }
        }
        Ok(())
    }
}

impl<'a> CombatLogReportIO for WowDeathEventsReportGenerator<'a> {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(dir.to_string());
        self.writer = Some(
            CombatLogAvroFileIO::new(dir, &REPORT_SCHEMA)?
        );
        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        let mut ret: Vec<Arc<dyn CombatLogReport + Send + Sync>> = vec![];
        for r in self.completed_death_recaps.drain(0..) {
            ret.push(r);
        }        

        if let Some(w) = self.writer.take() {
            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("deaths.avro"),
                    raw_file: RwLock::new(w.get_underlying_file()?),
                    canonical_type: WowReportTypes::Events as i32,
                })
            );
        }
        Ok(ret)
    }
}