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
        agg::{
            InputAggregatorPacket,
            CombatLogAggregator,
            sliding_window::{
                CombatLogSlidingWindowAggregator,
                SlidingWindowFunction,
            },
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
use std::time::Duration;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use avro_rs::{
    Schema,
};
use async_std::sync::{RwLock};

pub struct WowStatTimelineGenerator<'a> {
    writer: CombatLogAvroFileIO<'a>,
    agg: HashMap<String, CombatLogSlidingWindowAggregator<f64>>,
    start_tm: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all="camelCase")]
pub struct WowUnitTimelineEntry {
    pub guid: String,
    // tm is which time bucket we're in. E.g. if we're in the first 5 seconds, tm = 0, in the next 5 seconds, tm = 1, etc.
    pub tm: i64,
    pub value: f64,
}

const TIMELINE_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_stat_timeline",
        "fields": [
            {"name": "guid", "type": "string"},
            {"name": "tm", "type": "long"},
            {"name": "value", "type": "double"}
        ]
    }
"#;

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all="camelCase")]
pub struct WowUnitStatSummary {
    pub guid: String,
    pub damage_dealt: i64,
    pub damage_received: i64,
    pub heals: i64,
}

const SUMMARY_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_stat_summary",
        "fields": [
            {"name": "guid", "type": "string"},
            {"name": "damageDealt", "type": "long"},
            {"name": "damageReceived", "type": "long"},
            {"name": "heals", "type": "long"}
        ]
    }
"#;

lazy_static! {
    pub static ref TIMELINE_SCHEMA: Schema = Schema::parse_str(TIMELINE_SCHEMA_RAW).unwrap();
    pub static ref SUMMARY_SCHEMA: Schema = Schema::parse_str(SUMMARY_SCHEMA_RAW).unwrap();
}

const TIMELINE_BUCKET_DURATION_SECONDS: i64 = 5;

impl<'a> WowStatTimelineGenerator<'a> {
    fn new(work_dir: &str, start_tm: DateTime<Utc>) -> Result<Self, SquadOvError> {
        Ok(Self {
            writer: CombatLogAvroFileIO::new(work_dir, &TIMELINE_SCHEMA)?,
            agg: HashMap::new(),
            start_tm,
        })
    }

    fn ingest_data(&mut self, guid: &str, data: Option<InputAggregatorPacket<f64>>) -> Result<(), SquadOvError> {
        if !self.agg.contains_key(guid) {
            self.agg.insert(guid.to_string(), CombatLogSlidingWindowAggregator::new(
                SlidingWindowFunction::PerUnitTime(Duration::from_secs(1)),
                Duration::from_secs(TIMELINE_BUCKET_DURATION_SECONDS as u64),
                self.start_tm.clone(),
            ));
        }

        let agg = self.agg.get_mut(guid).unwrap();
        let packet = if let Some(d) = data {
            agg.handle(d.into())?
        } else {
            Some(agg.flush()?)
        };

        if let Some(packet) = packet {
            self.writer.handle(WowUnitTimelineEntry{
                guid: guid.to_string(),
                // The division and then multiplication is needed for when it isn't an exact multiple.
                tm: (packet.start - self.start_tm).num_seconds() / TIMELINE_BUCKET_DURATION_SECONDS * TIMELINE_BUCKET_DURATION_SECONDS,
                value: packet.value,
            })?;
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), SquadOvError> {
        let guids: Vec<String> = self.agg.keys().map(|x| { x.clone() }).collect();
        for guid in guids {
            self.ingest_data(&guid, None)?;
        }
        Ok(())
    }
}

pub struct WowStatReportGenerator<'a> {
    start_tm: DateTime<Utc>,
    work_dir: Option<String>,
    dps_timeline: Option<WowStatTimelineGenerator<'a>>,
    drps_timeline: Option<WowStatTimelineGenerator<'a>>,
    hps_timeline: Option<WowStatTimelineGenerator<'a>>,
    summary: HashMap<String, WowUnitStatSummary>,
    unit_ownership: HashMap<String, String>,
}

impl<'a> CombatLogReportHandler for WowStatReportGenerator<'a> {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    source,
                    dest,
                    event: WoWCombatLogEventType::DamageDone{amount, ..},
                    ..
                }
            } => {
                if let Some(src) = source {
                    self.add_damage_dealt_for_unit(timestamp.clone(), src.guid.as_str(), *amount)?;
                }

                if let Some(dst) = dest {
                    self.add_damage_received_for_unit(timestamp.clone(),dst.guid.as_str(), *amount)?;
                }
            },
            WowPacketData::Parsed{
                inner: WoWCombatLogEvent{
                    timestamp,
                    source: Some(src),
                    event: WoWCombatLogEventType::Healing{amount, overheal, ..},
                    ..
                }
            } => {
                self.add_heals_for_unit(timestamp.clone(),src.guid.as_str(), std::cmp::max(*amount - *overheal, 0))?;
            },
            _ => (),
        }

        Ok(())
    }
}

impl<'a> WowStatReportGenerator<'a> {
    pub fn new(start_tm: DateTime<Utc>) -> Self {
        Self {
            start_tm,
            work_dir: None,
            dps_timeline: None,
            drps_timeline: None,
            hps_timeline: None,
            summary: HashMap::new(),
            unit_ownership: HashMap::new(),
        }
    }

    pub fn update_ownership(&mut self, update: &HashMap<String, String>) {
        for (unit, owner) in update {
            self.unit_ownership.insert(unit.clone(), owner.clone());
        }
    }

    fn get_player_user_from_guid(&self, guid: &str) -> Option<String> {
        if let Some(owner) = self.unit_ownership.get(guid) {
            Some(owner.clone())
        } else if guid.starts_with("Player-") {
            Some(guid.to_string())
        } else {
            None
        }
    }

    fn add_damage_dealt_for_unit(&mut self, tm: DateTime<Utc>, unit: &str, damage: i64) -> Result<(), SquadOvError> {
        if let Some(unit) = self.get_player_user_from_guid(unit) {
            log::info!("add damage dealt: {} {}", &unit, damage);
            if let Some(summary) = self.summary.get_mut(&unit) {
                summary.damage_dealt += damage;
            } else {
                self.summary.insert(unit.clone(), WowUnitStatSummary{
                    guid: unit.clone(),
                    damage_dealt: damage,
                    ..WowUnitStatSummary::default()
                });
            }

            if let Some(timeline) = self.dps_timeline.as_mut() {
                timeline.ingest_data(unit.as_str(), Some(InputAggregatorPacket{
                    tm,
                    data: damage as f64,
                }))?;
            }
        }
        Ok(())
    }

    fn add_damage_received_for_unit(&mut self, tm: DateTime<Utc>, unit: &str, damage: i64) -> Result<(), SquadOvError> {
        if let Some(unit) = self.get_player_user_from_guid(unit) {
            if let Some(summary) = self.summary.get_mut(&unit) {
                summary.damage_received += damage;
            } else {
                self.summary.insert(unit.clone(), WowUnitStatSummary{
                    guid: unit.clone(),
                    damage_received: damage,
                    ..WowUnitStatSummary::default()
                });
            }

            if let Some(timeline) = self.drps_timeline.as_mut() {
                timeline.ingest_data(unit.as_str(), Some(InputAggregatorPacket{
                    tm,
                    data: damage as f64,
                }))?;
            }
        }
        Ok(())
    }

    fn add_heals_for_unit(&mut self, tm: DateTime<Utc>, unit: &str, amount: i64) -> Result<(), SquadOvError> {
        if let Some(unit) = self.get_player_user_from_guid(unit) {
            if let Some(summary) = self.summary.get_mut(&unit) {
                summary.heals += amount;
            } else {
                self.summary.insert(unit.clone(), WowUnitStatSummary{
                    guid: unit.clone(),
                    heals: amount,
                    ..WowUnitStatSummary::default()
                });
            }

            if let Some(timeline) = self.hps_timeline.as_mut() {
                timeline.ingest_data(unit.as_str(), Some(InputAggregatorPacket{
                    tm,
                    data: amount as f64,
                }))?;
            }
        }
        Ok(())
    }
}

impl<'a> CombatLogReportIO for WowStatReportGenerator<'a> {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        if let Some(tm) = self.dps_timeline.as_mut() {
            tm.flush()?;
        }

        if let Some(tm) = self.drps_timeline.as_mut() {
            tm.flush()?;
        }

        if let Some(tm) = self.hps_timeline.as_mut() {
            tm.flush()?;
        }

        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(dir.to_string());

        self.dps_timeline = Some(WowStatTimelineGenerator::new(dir, self.start_tm.clone())?);
        self.drps_timeline = Some(WowStatTimelineGenerator::new(dir, self.start_tm.clone())?);
        self.hps_timeline = Some(WowStatTimelineGenerator::new(dir, self.start_tm.clone())?);

        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        let mut ret: Vec<Arc<dyn CombatLogReport + Send + Sync>> = vec![];

        if let Some(gen) = self.dps_timeline.take() {
            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("dps.avro"),
                    raw_file: RwLock::new(gen.writer.get_underlying_file()?),
                    canonical_type: WowReportTypes::Stats as i32,
                })
            );
        }

        if let Some(gen) = self.drps_timeline.take() {
            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("drps.avro"),
                    raw_file: RwLock::new(gen.writer.get_underlying_file()?),
                    canonical_type: WowReportTypes::Stats as i32,
                })
            );
        }

        if let Some(gen) = self.hps_timeline.take() {
            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("hps.avro"),
                    raw_file: RwLock::new(gen.writer.get_underlying_file()?),
                    canonical_type: WowReportTypes::Stats as i32,
                })
            );
        }

        if let Some(work_dir) = self.work_dir.as_ref() {
            let mut w = CombatLogAvroFileIO::new(work_dir, &SUMMARY_SCHEMA)?;
            for (_, summary) in self.summary.drain() {
                w.handle(summary)?;
            }

            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("summary.avro"),
                    raw_file: RwLock::new(w.get_underlying_file()?),
                    canonical_type: WowReportTypes::Stats as i32,
                })
            );
        }
        
        Ok(ret)
    }
}