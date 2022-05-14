mod cache;

use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReportHandler,
        CombatLogReportIO,
        RawStaticCombatLogReport,
        io::{
            CombatLogDiskIO,
            avro::CombatLogAvroFileIO,
            json::CombatLogJsonFileIO,
        },
        CombatLogReport,
        CombatLog,
    },
    wow::{
        combatlog::{
            WowCombatLogPacket,
            WowPacketData,
            WoWCombatLogSourceDest,
            WoWCombatLogEventType,
        },
        reports::WowReportTypes,
        WowFullCharacter,
        constants,
        characters::{
            self,
            WowItem,
            WowCovenant,
        },
    },
};
use serde::Serialize;
use avro_rs::{
    Schema,
};
use async_std::sync::{RwLock};
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use sqlx::{
    postgres::{PgPool},
};

pub struct WowCharacterReportGenerator {
    parent_cl: CombatLog,
    work_dir: Option<String>,
    chars: HashMap<String, WowCharacterReport>,
    self_guid: Option<String>,
    per_combatant_unique_spells: HashMap<String, HashSet<i64>>,
    combatants: HashMap<String, WowCombatantReport>,
    loadouts: HashMap<String, WowFullCharacter>,
    pool: Arc<PgPool>,
    build_version: String,
    // key: unit, value: owner
    ownership_updates: HashMap<String, String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowCharacterReport {
    unit_guid: String,
    unit_name: String,
    flags: HashSet<i64>,
    owner_guid: Option<String>,
}

const CHAR_REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_character_summary",
        "fields": [
            {"name": "unitGuid", "type": "string"},
            {"name": "unitName", "type": "string"},
            {"name": "flags", "type": {
                "type": "array",
                "items": "long",
                "default": []
            }},
            {"name": "ownerGuid", "type": ["null", "string"]}
        ]
    }
"#;

#[derive(Serialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct WowCombatantReport {
    unit_guid: String,
    unit_name: String,
    ilvl: i32,
    spec_id: i32,
    team: i32,
    rating: i32,
    class_id: Option<i64>,
}

const COMBATANT_REPORT_SCHEMA_RAW: &'static str = r#"
    {
        "type": "record",
        "name": "wow_combatant_summary",
        "fields": [
            {"name": "unitGuid", "type": "string"},
            {"name": "unitName", "type": "string"},
            {"name": "ilvl", "type": "int"},
            {"name": "specId", "type": "int"},
            {"name": "team", "type": "int"},
            {"name": "rating", "type": "int"},
            {"name": "classId", "type": ["null", "long"]}
        ]
    }
"#;

lazy_static! {
    static ref CHAR_REPORT_SCHEMA: Schema = Schema::parse_str(CHAR_REPORT_SCHEMA_RAW).unwrap();
    static ref COMBATANT_REPORT_SCHEMA: Schema = Schema::parse_str(COMBATANT_REPORT_SCHEMA_RAW).unwrap();
}

impl CombatLogReportHandler for WowCharacterReportGenerator {
    type Data = WowCombatLogPacket;
    fn handle(&mut self, data: &Self::Data) -> Result<(), SquadOvError> {
        match &data.data {
            WowPacketData::Parsed{inner} => {
                // Both SOURCE and DEST characters can be a valid source for finding a character that we need to keep track of.
                if let Some(src) = &inner.source {
                    self.ingest_basic_character(src)?;
                }

                if let Some(dst) = &inner.dest {
                    self.ingest_basic_character(dst)?;
                }

                match &inner.event {
                    WoWCombatLogEventType::CombatantInfo{guid, team, spec_id, talents, pvp_talents, covenant, items, rating, ..} => {
                        // The other source of finding character information is the COMBATANT_INFO line.
                        self.initialize_combatant(guid, "")?;

                        if let Some(c) = self.combatants.get_mut(guid) {
                            c.ilvl = characters::compute_wow_character_ilvl(&items.iter().map(|x| { x.ilvl }).collect::<Vec<_>>());
                            c.spec_id = *spec_id;
                            c.team = *team;
                            c.rating = *rating;
                        }

                        if let Some(l) = self.loadouts.get_mut(guid) {
                            l.items = items.iter().map(|x| {
                                WowItem{
                                    item_id: x.item_id,
                                    ilvl: x.ilvl,
                                }
                            }).collect();
                            l.covenant = covenant.as_ref().map(|x| {
                                WowCovenant{
                                    covenant_id: x.covenant_id,
                                    soulbind_id: x.soulbind_id,
                                    soulbind_traits: x.soulbind_traits.clone(),
                                    conduits: x.conduits.iter().map(|x| {
                                        WowItem{
                                            item_id: x.item_id,
                                            ilvl: x.ilvl,
                                        }
                                    }).collect(),
                                }
                            });
                            l.talents = talents.clone();
                            l.pvp_talents = pvp_talents.clone();
                        }
                    },
                    WoWCombatLogEventType::SpellCast{spell, ..} => {
                        // We also want to guesstimate the combatant's class in the case of a spell cast.
                        // Instead of hammering out database with a shit ton of requests, we bulk these requests up and fire it off
                        // all at once at the end in finalize.
                        if let Some(src) = &inner.source {
                            if !self.per_combatant_unique_spells.contains_key(&src.guid) {
                                self.per_combatant_unique_spells.insert(src.guid.clone(), HashSet::new());
                            }

                            let spells = self.per_combatant_unique_spells.get_mut(&src.guid).unwrap();
                            spells.insert(spell.id); 
                        }
                    },
                    WoWCombatLogEventType::SpellSummon{..} => {
                        // Summoning a character is an implicit ownership event.
                        if let Some(src) = &inner.source {
                            if let Some(dst) = &inner.dest {
                                self.mark_ownership(&dst.guid, &src.guid)?;
                            }
                        }
                    },
                    _ => (),
                }

                if let Some(adv) = &inner.advanced {
                    if adv.owner_guid != constants::NIL_WOW_GUID && adv.unit_guid != constants::NIL_WOW_GUID {
                        self.mark_ownership(&adv.unit_guid, &adv.owner_guid)?;
                    }
                }
            },
            _ => (),
        }
        Ok(())
    }
}

impl WowCharacterReportGenerator {
    pub fn new(pool: Arc<PgPool>, parent_cl: CombatLog, build_version: String) -> Self {
        WowCharacterReportGenerator{
            parent_cl,
            work_dir: None,
            chars: HashMap::new(),
            self_guid: None,
            per_combatant_unique_spells: HashMap::new(),
            combatants: HashMap::new(),
            loadouts: HashMap::new(),
            pool,
            build_version,
            ownership_updates: HashMap::new(),
        }
    }

    pub fn get_ownership_update(&mut self) -> HashMap<String, String> {
        self.ownership_updates.drain().collect()
    }

    fn mark_ownership(&mut self, unit_guid: &str, owner_guid: &str) -> Result<(), SquadOvError> {
        if let Some(c) = self.chars.get_mut(unit_guid) {
            c.owner_guid = Some(owner_guid.to_string());
            self.ownership_updates.insert(unit_guid.to_string(), owner_guid.to_string());
        }
        Ok(())
    }

    fn initialize_combatant(&mut self, guid: &str, name: &str) -> Result<(), SquadOvError> {
        if let Some(c) = self.combatants.get_mut(guid) {
            c.unit_name = name.to_string();
        } else {
            self.combatants.insert(guid.to_string(), WowCombatantReport{
                unit_guid: guid.to_string(),
                unit_name: name.to_string(),
                ilvl: 0,
                spec_id: 0,
                team: 0,
                rating: 0,
                class_id: None,
            });
        }

        if !self.loadouts.contains_key(guid) {
            self.loadouts.insert(guid.to_string(), WowFullCharacter{
                items: vec![],
                covenant: None,
                talents: vec![],
                pvp_talents: vec![],
            });
        }
        Ok(())
    }

    fn ingest_basic_character(&mut self, data: &WoWCombatLogSourceDest) -> Result<(), SquadOvError> {
        // All characters need to be tracked with character reports.
        if let Some(cr) = self.chars.get_mut(&data.guid) {
            cr.flags.insert(data.flags);
        } else {
            self.chars.insert(data.guid.clone(), WowCharacterReport{
                unit_guid: data.guid.clone(),
                unit_name: data.name.clone(),
                flags: HashSet::from_iter(vec![data.flags]),
                owner_guid: None,
            });
        }

        // All players needs to be tracked with combatant reports.
        if data.guid.starts_with("Player-") {
            self.initialize_combatant(&data.guid, &data.name)?;

            if data.flags & constants::COMBATLOG_FILTER_ME == constants::COMBATLOG_FILTER_ME && data.guid != constants::NIL_WOW_GUID {
                self.self_guid = Some(data.guid.clone());
            }
        }

        Ok(())
    }
}

impl CombatLogReportIO for WowCharacterReportGenerator {
    fn finalize(&mut self) -> Result<(), SquadOvError> {
        // We need to figure out what classes all these players are (in the per_combatant_unique_spells hashmap)
        // from the database based on what spells they cast. Is this reliable? Nah but good enough-ish?
        let mut all_spell_ids: Vec<i32> = vec![];
        for (_, spell_ids) in &self.per_combatant_unique_spells {
            for s in spell_ids {
                all_spell_ids.push(*s as i32);
            }
        }

        let handle = tokio::runtime::Handle::current();
        let build_version = self.build_version.clone();
        let pool = self.pool.clone();
        let spell_to_class = handle.block_on(async move {
            Ok::<HashMap<i32, i32>, SquadOvError>(
                sqlx::query!(
                    "
                    SELECT spell_id, class_id
                    FROM squadov.wow_spell_to_class
                    WHERE spell_id = ANY($1)
                        AND $2 LIKE build_id || '.%'
                    ",
                    &all_spell_ids,
                    &build_version,
                )
                    .fetch_all(&*pool)
                    .await?
                    .into_iter()
                    .map(|x| {
                        (x.spell_id, x.class_id)
                    })
                    .collect::<HashMap<i32, i32>>()
            )
        })?;
        
        // For each combatant, come to a consensus on what class they are based on what spells they cast.
        for (guid, spell_ids) in &self.per_combatant_unique_spells {
            let mut votes: HashMap<i32, i32> = HashMap::new();
            for s in spell_ids {
                if let Some(c) = spell_to_class.get(&(*s as i32)) {
                    votes.insert(*c, votes.get(c).map(|x| *x).unwrap_or(0) + 1);
                }
            }

            if votes.is_empty() {
                continue;
            }

            let voted_class = votes.into_iter().max_by(|a, b| a.1.cmp(&b.1)).map(|(k, _v)| k);
            if let Some(combatant) = self.combatants.get_mut(guid) {
                combatant.class_id = voted_class.map(|x| x as i64);
            }
        }

        Ok(())
    }

    fn initialize_work_dir(&mut self, dir: &str) -> Result<(), SquadOvError> {
        self.work_dir = Some(dir.to_string());
        Ok(())
    }

    fn get_reports(&mut self) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        let mut ret: Vec<Arc<dyn CombatLogReport + Send + Sync>> = vec![];
        let work_dir = self.work_dir.as_ref().ok_or(SquadOvError::InternalError(String::from("No Work Dir Set [WowCharacterReportGenerator]")))?.as_str();

        // Create a report that associates the report owner with the current character.
        if let Some(self_guid) = self.self_guid.take() {
            let mut report = cache::WowUserCharacterCacheReport{
                user_id: self.parent_cl.owner_id,
                build_version: self.build_version.clone(),
                unit_guid: self_guid.clone(),
                ..cache::WowUserCharacterCacheReport::default()
            };

            if let Some(ch) = self.chars.get(&self_guid) {
                report.unit_name = ch.unit_name.clone();
            }

            if let Some(ct) = self.combatants.get(&self_guid) {
                report.spec_id = ct.spec_id;
                report.class_id = ct.class_id.clone().map(|x| { x as i32 });
            }

            if let Some(loadout) = self.loadouts.get(&self_guid) {
                report.items = loadout.items.iter().map(|x| { x.item_id as i32 }).collect();
            }

            ret.push(Arc::new(report));
        }

        {
            let mut w = CombatLogAvroFileIO::new(work_dir, &CHAR_REPORT_SCHEMA)?;
            for (_, c) in self.chars.drain() {
                log::info!("Add char: {:?}", &c);
                w.handle(c)?;
            }

            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("characters.avro"),
                    raw_file: RwLock::new(w.get_underlying_file()?),
                    canonical_type: WowReportTypes::MatchCharacters as i32,
                })
            );
        }

        {
            let mut w = CombatLogAvroFileIO::new(work_dir, &COMBATANT_REPORT_SCHEMA)?;
            for (_, c) in self.combatants.drain() {
                log::info!("Add combatant: {:?}", &c);
                w.handle(c)?;
            }

            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: String::from("combatants.avro"),
                    raw_file: RwLock::new(w.get_underlying_file()?),
                    canonical_type: WowReportTypes::MatchCombatants as i32,
                })
            );
        }

        for (key, loadout) in self.loadouts.drain() {
            let mut w = CombatLogJsonFileIO::new(work_dir)?;
            w.handle(loadout)?;

            ret.push(
                Arc::new(RawStaticCombatLogReport{
                    key_name: format!("{}.json", key),
                    raw_file: RwLock::new(w.get_underlying_file()?),
                    canonical_type: WowReportTypes::CharacterLoadout as i32,
                })
            );
        }
        
        Ok(ret)
    }
}