use serde::Deserialize;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use uuid::Uuid;
use std::convert::TryInto;
use std::cmp::PartialEq;
use std::str::FromStr;
use crate::SquadOvError;
use unicode_segmentation::UnicodeSegmentation;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct WoWCombatLogState {
    #[serde(rename="combatLogVersion")]
    pub combat_log_version: String,
    #[serde(rename="advancedLog")]
    pub advanced_log: bool,
    #[serde(rename="buildVersion")]
    pub build_version: String
}

pub struct FullWoWCombatLogState {
    pub state: WoWCombatLogState,
    pub blob: crate::BlobResumableIdentifier,
}

#[derive(Deserialize)]
pub struct RawWoWCombatLogPayload {
    pub timestamp: DateTime<Utc>,
    pub parts: Vec<String>,
    #[serde(rename="logLine")]
    pub log_line: i64,
}

fn split_wow_combat_log_tokens(full_log: &str) -> Vec<String> {
    let mut new_parts: Vec<String> = Vec::new();

    let log_parts: Vec<&str> = full_log.graphemes(true).collect();
    let mut log_iter = log_parts.iter();

    let mut stack: Vec<&str> = vec![","];
    let mut current_part: Vec<&str> = Vec::new();
    while let Some(ch) = log_iter.next() {
        // There are generally 3 cases here when we first start off (i.e.
        // we're coming clean off processing an ending comma)
        // 1) We encounter a [ or a (
        // 2) We encounter a "
        // 3) We encounter any other character.
        // In cases 1 and 3, we want to grab the string that goes from the
        // current position up to the character before the relevant closing comma.
        // In case 2, we want to grab the string that goes from the next position up
        // to the character before the closing quotation.
        //
        // To accomplish this, we keep track of a "stack" of special symbols that will
        // represent some sort of a nested string, i.e. [], (), "". We also keep track of
        // all top-level commas in the stack (a top-level comma is defined to be a comma
        // that is detected when the stack is empty). Each comma will perform a pop AND a push
        // operation as it simultaneously marks the ending of the previous stack and the start
        // of a new stack. We only detect a new part when the stack is empty.
        //
        // For example, assume are we given the following string
        //
        //     ["TEST"],
        //
        // We implicitly assume that the string starts with a comma to begin the stack.
        // We detect a [ so push it onto the stack. Then we detect a " and we push it onto
        // the stack (only the [ gets tracked as part of the current part). Each character
        // of TEST is non-special and thus collected as part of the current part (i.e [TEST ).
        // Finally, we detect " and then ] which pop " and [ off the stack respectively. Finally
        // we detect the final comma which pops the first comma off the stack so that we end up
        // with a part of [TEST]. The only cases where a special character is not added to the
        // current part is in the case of a top-level comma and quotation marks.

        match *ch {
            "[" | "(" => {
                current_part.push(ch);
                stack.push(ch);
            },
            "]" | ")" => {
                let last = stack.pop().unwrap_or("");
                if (*ch == "]" && last != "[") || (*ch == ")" && last != "(") {
                    log::warn!("WoW Log Mismatched Char: {} vs {} - {}", ch, last, full_log);
                    break;
                }
                current_part.push(ch)
            },
            "\"" => {
                // Note that we need to differentiate between quotation marks that
                // start a stack and quotation marks that end a stack. If the current
                // element on the stack isn't a quotation mark then we assume that it
                // starts the stack and if it is a quotation mark then we assume that
                // it ends the stack. This assumption rests on the fact that I believe
                // that it isn't possible for any string in the combat log to look like
                // "\"".
                let last = stack.last();
                if last.is_none() {
                    log::warn!("Empty stack.");
                    break;
                }

                if last.unwrap() == ch {
                    stack.pop();
                } else {
                    stack.push(ch);
                }
            },
            "," => {
                if stack.len() == 1 && stack[0] == "," {
                    new_parts.push(String::from(current_part.join("")));
                    current_part.clear();
                } else {
                    current_part.push(ch)
                }
            },
            _ => current_part.push(ch)
        }          
    }

    // Implicit comma at the end of the string
    new_parts.push(String::from(current_part.join("")));
    new_parts
}

impl RawWoWCombatLogPayload {
    pub fn flatten(&self) -> String {
        format!(
            "{} {}\n",
            self.timestamp.format("%m/%d %T").to_string(),
            self.parts.join(","),
        )
    }

    pub fn redo_parts(&mut self) {
        // I'm not really sure if we need to do unicode aware stepping here
        // but it's probably better safe than sorry.
        let full_log = self.parts.join(",");
        self.parts = split_wow_combat_log_tokens(&full_log);
    }

    pub fn is_finish_token(&self) -> bool {
        self.parts.len() > 0 && self.parts[0] == "SQUADOV_END_COMBAT_LOG"
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct WoWSpellInfo {
    id: i64,
    name: String,
    school: i64
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum WoWDamageType {
    SwingDamage,
    SpellDamage(WoWSpellInfo)
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum WoWSpellAuraType {
    Buff,
    Debuff,
    Unknown
}

impl FromStr for WoWSpellAuraType {
    type Err = SquadOvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BUFF" => Ok(WoWSpellAuraType::Buff),
            "DEBUFF" => Ok(WoWSpellAuraType::Debuff),
            _ => Err(SquadOvError::BadRequest)
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct WoWItemInfo {
    item_id: i64,
    ilvl: i32
}

fn parse_wow_item_info_from_str(s: &str) -> Result<Vec<WoWItemInfo>, SquadOvError> {
    let tokens = split_wow_combat_log_tokens(&s[1..s.len()-1]);
    // Each top level token is for one given item.
    Ok(tokens.into_iter().map(|x| {
        let item_parts = split_wow_combat_log_tokens(&x[1..x.len()-1]);
        WoWItemInfo{
            item_id: item_parts[0].parse().unwrap_or(-1),
            ilvl: item_parts[1].parse().unwrap_or(-1),
        }
    }).collect())
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum WoWCombatLogEventType {
    UnitDied,
    DamageDone{
        damage: WoWDamageType,
        amount: i64,
        overkill: i64,
    },
    Healing{
        spell: WoWSpellInfo,
        amount: i64,
        overheal: i64,
        absorbed: i64,
    },
    Resurrect(WoWSpellInfo),
    SpellAura{
        spell: WoWSpellInfo,
        aura_type: WoWSpellAuraType,
        applied: bool
    },
    SpellSummon(WoWSpellInfo),
    CombatantInfo{
        guid: String,
        strength: i32,
        agility: i32,
        stamina: i32,
        intelligence: i32,
        armor: i32,
        spec_id: i32,
        items: Vec<WoWItemInfo>,
    },
    EncounterStart{
        encounter_id: i32,
        encounter_name: String,
        difficulty: i32,
        num_players: i32,
        instance_id: i32
    },
    EncounterEnd{
        encounter_id: i32,
        encounter_name: String,
        difficulty: i32,
        num_players: i32,
        success: bool
    },
    ChallengeModeStart{
        challenge_name: String,
        instance_id: i32,
        keystone: i32
    },
    ChallengeModeEnd{
        instance_id: i32,
        success: bool,
        keystone: i32,
        time_ms: i64
    },
    Unknown
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct WoWCombatLogSourceDest {
    guid: String,
    name: String,
    flags: i64,
    raid_flags: i64,
}

impl WoWCombatLogSourceDest {
    fn new(data: &[String; 4]) -> Result<Self, crate::SquadOvError> {
        Ok(Self {
            guid: data[0].clone(),
            name: data[1].clone(),
            flags: i64::from_str_radix(&data[2][2..], 16)?,
            raid_flags: i64::from_str_radix(&data[3][2..], 16)?,
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum WoWGenericLevel {
    ItemLevel{
        level: i32
    },
    UnitLevel{
        level: i32
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag="type")]
pub struct WoWCombatLogAdvancedCVars {
    unit_guid: String,
    owner_guid: String,
    current_hp: i64,
    max_hp: i64,
    attack_power: i64,
    spell_power: i64,
    armor: i64,
    unk1: i64,
    resource_type: i32,
    current_resource: i64,
    max_resource: i64,
    resource_cost: i64,
    coord0: f64,
    coord1: f64,
    map_id: i64,
    facing: f64,
    level: WoWGenericLevel
}

impl WoWCombatLogAdvancedCVars {
    fn new(data: &[String; 17], use_ilvl: bool) -> Result<Self, crate::SquadOvError> {
        let level: i32 = data[16].parse()?;
        Ok(Self {
            unit_guid: data[0].clone(),
            owner_guid: data[1].clone(),
            current_hp: data[2].parse()?,
            max_hp: data[3].parse()?,
            attack_power: data[4].parse()?,
            spell_power: data[5].parse()?,
            armor: data[6].parse()?,
            unk1: data[7].parse()?,
            resource_type: data[8].parse()?,
            current_resource: data[9].parse()?,
            max_resource: data[10].parse()?,
            resource_cost: data[11].parse()?,
            coord0: data[12].parse()?,
            coord1: data[13].parse()?,
            map_id: data[14].parse()?,
            facing: data[15].parse()?,
            level: if use_ilvl {
                WoWGenericLevel::ItemLevel{level}
            } else {
                WoWGenericLevel::UnitLevel{level}
            }
        })
    }
}

#[derive(Clone)]
pub struct WoWCombatLogEvent {
    combat_log_id: Uuid,
    log_line: i64,
    timestamp: DateTime<Utc>,
    source: Option<WoWCombatLogSourceDest>,
    dest: Option<WoWCombatLogSourceDest>,
    advanced: Option<WoWCombatLogAdvancedCVars>,
    pub event: WoWCombatLogEventType
}

pub fn parse_advanced_cvars_and_event_from_wow_combat_log(state: &WoWCombatLogState, payload: &RawWoWCombatLogPayload) -> Result<(Option<WoWCombatLogAdvancedCVars>, WoWCombatLogEventType), crate::SquadOvError> {
    let action_parts: Vec<&str> = payload.parts[0].split("_").collect();
    
    match action_parts[0] {
        "SPELL" => {
            match action_parts[1] {
                // Need to handle ABSORBED separately since its format is different from other spells.
                "ABSORBED" => Ok((None, WoWCombatLogEventType::Unknown)),
                _ => {
                    let mut idx = 9;
                    let spell_info = WoWSpellInfo{
                        id: payload.parts[idx].parse()?,
                        name: payload.parts[idx+1].clone(),
                        school: i64::from_str_radix(&payload.parts[idx+2][2..], 16)?,
                    };
                    idx += 3;
                    
                    match payload.parts[0].as_str() {
                        "SPELL_DAMAGE" | "SPELL_PERIODIC_DAMAGE" => Ok({
                            let advanced = if state.advanced_log {
                                Some(WoWCombatLogAdvancedCVars::new(payload.parts[idx..idx+17].try_into()?, payload.parts[0].as_str() == "SPELL_DAMAGE")?)
                            } else {
                                None
                            };
                            idx += 17;
        
                            (advanced, WoWCombatLogEventType::DamageDone{
                                damage: WoWDamageType::SpellDamage(spell_info),
                                amount: payload.parts[idx].parse()?,
                                overkill: payload.parts[idx+1].parse()?,    
                            })
                        }),
                        "SPELL_HEAL" | "SPELL_PERIODIC_HEAL" => Ok({
                            let advanced = if state.advanced_log {
                                Some(WoWCombatLogAdvancedCVars::new(payload.parts[idx..idx+17].try_into()?, true)?)
                            } else {
                                None
                            };
                            idx += 17;
        
                            (advanced, WoWCombatLogEventType::Healing{
                                spell: spell_info,
                                amount: payload.parts[idx+1].parse()?,
                                overheal: payload.parts[idx+2].parse()?,
                                absorbed: payload.parts[idx+3].parse()?,
                            })
                        }),
                        "SPELL_RESURRECT" => Ok((None, WoWCombatLogEventType::Resurrect(spell_info))),
                        "SPELL_AURA_APPLIED" => Ok((None, WoWCombatLogEventType::SpellAura{
                            spell: spell_info,
                            aura_type: WoWSpellAuraType::from_str(&payload.parts[idx])?,
                            applied: true
                        })),
                        "SPELL_AURA_REMOVED" => Ok((None, WoWCombatLogEventType::SpellAura{
                            spell: spell_info,
                            aura_type: WoWSpellAuraType::from_str(&payload.parts[idx])?,
                            applied: false
                        })),
                        "SPELL_SUMMON" => Ok((None, WoWCombatLogEventType::SpellSummon(spell_info))),
                        _ => Ok((None, WoWCombatLogEventType::Unknown)),
                    }
                }
            }
        },
        _ => match payload.parts[0].as_str() {
            "UNIT_DIED" => Ok((None, WoWCombatLogEventType::UnitDied)),
            "SWING_DAMAGE_LANDED" => Ok({
                let mut idx = 9;
                let advanced = if state.advanced_log {
                    Some(WoWCombatLogAdvancedCVars::new(payload.parts[idx..idx+17].try_into()?, false)?)
                } else {
                    None
                };
                idx += 17;

                (advanced, WoWCombatLogEventType::DamageDone{
                    damage: WoWDamageType::SwingDamage,
                    amount: payload.parts[idx].parse()?,
                    overkill: payload.parts[idx+1].parse()?,
                })
            }),
            "COMBATANT_INFO" => Ok((None, WoWCombatLogEventType::CombatantInfo{
                guid: payload.parts[1].clone(),
                strength: payload.parts[3].parse()?,
                agility: payload.parts[4].parse()?,
                stamina: payload.parts[5].parse()?,
                intelligence: payload.parts[6].parse()?,
                armor: payload.parts[23].parse()?,
                spec_id: payload.parts[24].parse()?,
                items: parse_wow_item_info_from_str(&payload.parts[28])?,
            })),
            "ENCOUNTER_START" => Ok((None, WoWCombatLogEventType::EncounterStart{
                encounter_id: payload.parts[1].parse()?,
                encounter_name: payload.parts[2].clone(),
                difficulty: payload.parts[3].parse()?,
                num_players: payload.parts[4].parse()?,
                instance_id: payload.parts[5].parse()?,
            })),
            "ENCOUNTER_END" => Ok((None, WoWCombatLogEventType::EncounterEnd{
                encounter_id: payload.parts[1].parse()?,
                encounter_name: payload.parts[2].clone(),
                difficulty: payload.parts[3].parse()?,
                num_players: payload.parts[4].parse()?,
                success: payload.parts[5] == "1"
            })),
            "CHALLENGE_MODE_START" => Ok((None, WoWCombatLogEventType::ChallengeModeStart{
                challenge_name: payload.parts[1].clone(),
                instance_id: payload.parts[2].parse()?,
                keystone: payload.parts[4].parse()?,
            })),
            "CHALLENGE_MODE_END" => Ok((None, WoWCombatLogEventType::ChallengeModeEnd{
                instance_id: payload.parts[1].parse()?,
                success: payload.parts[2] == "1",
                keystone: payload.parts[3].parse()?,
                time_ms: payload.parts[4].parse()?,
            })),
            _ => Ok((None, WoWCombatLogEventType::Unknown))
        },
    }
}

pub fn parse_raw_wow_combat_log_payload(uuid: &Uuid, state: &WoWCombatLogState, payload: &RawWoWCombatLogPayload) -> Result<Option<WoWCombatLogEvent>, crate::SquadOvError> {
    let (advanced, event) = parse_advanced_cvars_and_event_from_wow_combat_log(state, payload)?;
    if event == WoWCombatLogEventType::Unknown {
        return Ok(None)
    }

    let has_source_dest = match payload.parts[0].as_str() {
        "COMBAT_LOG_VERSION" |
            "ZONE_CHANGE" |
            "MAP_CHANGE" |
            "ENCOUNTER_START" |
            "ENCOUNTER_END" |
            "CHALLENGE_MODE_START" |
            "CHALLENGE_MODE_END" |
            "COMBATANT_INFO" => false,
        _ => true,
    };

    Ok(Some(WoWCombatLogEvent{
        combat_log_id: uuid.clone(),
        log_line: payload.log_line,
        timestamp: payload.timestamp.clone(),
        source: if has_source_dest { Some(WoWCombatLogSourceDest::new(payload.parts[1..5].try_into()?)?) } else { None },
        dest: if has_source_dest { Some(WoWCombatLogSourceDest::new(payload.parts[5..9].try_into()?)?) } else { None },
        advanced,
        event
    }))
}

pub type WowCombatLogUnitOwnershipMapping = HashMap<Uuid, (String, String)>;
pub type WowCombatLogCharacterOwnershipMapping = HashMap<Uuid, String>;

pub async fn store_combat_log_unit_ownership_mapping<'a, T>(ex: &'a mut T, mapping: &WowCombatLogUnitOwnershipMapping) -> Result<(), crate::SquadOvError>
where
    &'a mut T: Executor<'a, Database = Postgres>
{
    if mapping.is_empty() {
        return Ok(());
    }

    let mut values: Vec<String> = Vec::new();
    for (combatlog, (unit_guid, owner_guid)) in mapping {
        values.push(format!(
            "('{combat_log_uuid}', '{unit_guid}', '{owner_guid}')",
            combat_log_uuid=combatlog.to_string(),
            unit_guid=unit_guid,
            owner_guid=owner_guid,
        ));
    }

    sqlx::query(&format!(
        "
        INSERT INTO squadov.wow_combatlog_unit_ownership (combat_log_uuid, unit_guid, owner_guid)
        VALUES {values}
        ON CONFLICT DO NOTHING
        ",
        values=values.join(",")
    ))
        .execute(ex)
        .await?;

    Ok(())
}

pub async fn store_combat_log_user_character_mapping<'a, T>(ex: &'a mut T, mapping: &WowCombatLogCharacterOwnershipMapping) -> Result<(), crate::SquadOvError>
where
    &'a mut T: Executor<'a, Database = Postgres>
{
    if mapping.is_empty() {
        return Ok(());
    }

    let mut values: Vec<String> = Vec::new();
    for (combatlog, guid) in mapping {
        values.push(format!(
            "('{combatlog}', '{guid}')",
            combatlog=combatlog.to_string(),
            guid=guid
        ));
    }

    sqlx::query(&format!(
        "
        INSERT INTO squadov.wow_user_character_association (user_id, guid)
        SELECT wcl.user_id, v.guid
        FROM (VALUES {values}) AS v(combatlog_id, guid)
        INNER JOIN squadov.wow_combat_logs AS wcl
            ON wcl.uuid = v.combatlog_id::UUID
        ON CONFLICT DO NOTHING
        ",
        values=values.join(",")
    ))
        .execute(ex)
        .await?;

    Ok(())
}

pub async fn store_wow_combat_log_events<'a, T>(ex: &'a mut T, events: &[WoWCombatLogEvent]) -> Result<(WowCombatLogUnitOwnershipMapping, WowCombatLogCharacterOwnershipMapping), crate::SquadOvError>
where
    &'a mut T: Executor<'a, Database = Postgres>
{
    if events.is_empty() {
        return Ok((HashMap::new(), HashMap::new()));
    }

    let mut sql: Vec<String> = Vec::new();
    sql.push(String::from("
        INSERT INTO squadov.wow_combat_log_events (
            combat_log_uuid,
            log_line,
            tm,
            source,
            dest,
            advanced,
            evt
        )
        VALUES
    "));

    let mut combatlog_current_players: WowCombatLogCharacterOwnershipMapping = HashMap::new();
    let mut combatlog_ownership: WowCombatLogUnitOwnershipMapping = HashMap::new();

    for eve in events {
        sql.push(format!("(
            '{uuid}',
            {log_line},
            {tm},
            {source},
            {dest},
            {advanced},
            {evt}
        )",
            uuid=&eve.combat_log_id,
            log_line=eve.log_line,
            tm=crate::sql_format_time(&eve.timestamp),
            source=crate::sql_format_option_json(&eve.source)?,
            dest=crate::sql_format_option_json(&eve.dest)?,
            advanced=crate::sql_format_option_json(&eve.advanced)?,
            evt=crate::sql_format_json(&eve.event)?,
        ));
        sql.push(String::from(","));

        // If source/dest is not none then we need to check the flags on the
        // relevant object and extract that information if necessary. In 
        // particular, we're interested in whether or not this player is the
        // "current player" for the uploader of the combat log.
        if eve.source.is_some() {
            let source = eve.source.as_ref().unwrap();
            if source.flags & crate::COMBATLOG_FILTER_ME == crate::COMBATLOG_FILTER_ME && source.guid != crate::NIL_WOW_GUID {
                combatlog_current_players.insert(eve.combat_log_id.clone(), source.guid.clone());
            }
        }

        if eve.dest.is_some() {
            let dest = eve.source.as_ref().unwrap();
            if dest.flags & crate::COMBATLOG_FILTER_ME == crate::COMBATLOG_FILTER_ME && dest.guid != crate::NIL_WOW_GUID {
                combatlog_current_players.insert(eve.combat_log_id.clone(), dest.guid.clone());
            }
        }

        if eve.advanced.is_some() {
            let advanced = eve.advanced.as_ref().unwrap();
            if advanced.owner_guid != crate::NIL_WOW_GUID && advanced.unit_guid != crate::NIL_WOW_GUID {
                combatlog_ownership.insert(eve.combat_log_id.clone(), (advanced.unit_guid.clone(), advanced.owner_guid.clone()));
            }
        }

        match eve.event {
            WoWCombatLogEventType::SpellSummon(_) => {
                if eve.source.is_some() && eve.dest.is_some() {
                    let source = eve.source.as_ref().unwrap();
                    let dest = eve.dest.as_ref().unwrap();
                    combatlog_ownership.insert(eve.combat_log_id.clone(), (dest.guid.clone(), source.guid.clone()));
                }
            }
            _ => ()
        }
    }

    sql.truncate(sql.len() - 1);
    sql.push(String::from(" ON CONFLICT DO NOTHING"));
    sqlx::query(&sql.join("")).execute(ex).await?;
    Ok((combatlog_ownership, combatlog_current_players))
}

#[cfg(test)]
mod tests {
    use super::*;

    extern crate env_logger;

    fn init() {
        std::env::set_var("RUST_LOG", "debug");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_is_finish_token() {
        let t1 = RawWoWCombatLogPayload{
            timestamp: Utc::now(),
            parts: vec![String::from("Test"), String::from("1")],
            log_line: 1
        };
        assert_eq!(t1.is_finish_token(), false);

        let t2 = RawWoWCombatLogPayload{
            timestamp: Utc::now(),
            parts: vec![String::from("SQUADOV_END_COMBAT_LOG")],
            log_line: 1
        };
        assert_eq!(t2.is_finish_token(), true);
    }

    #[test]
    fn test_split_wow_combat_log_tokens() {
        init();

        struct TestDatum {
            input: &'static str,
            output: Vec<&'static str>,
        };

        let test_data = vec![
            TestDatum{
                input: r#"CHALLENGE_MODE_START,"Plaguefall",2289,379,2,[9]"#,
                output: vec![
                    "CHALLENGE_MODE_START",
                    "Plaguefall",
                    "2289",
                    "379",
                    "2",
                    "[9]"
                ],
            },
            TestDatum{
                input: r#"COMBATANT_INFO,Player-3685-0CF2175A,0,1042,155,1502,448,0,0,0,307,307,307,0,0,518,518,518,0,128,495,495,495,1038,70,(269569,326732,234299,230332,223817,326734,317866),(0,210256,246806,305394),[7,1,[],[(1305),(1312),(1316),(1308),(1310)],[(216,158),(129,184)]],[(178777,184,(),(6807,6652,7194,1498,6646),()),(175885,184,(),(6789,1498,6646),()),(175872,184,(),(6782,1498,6646),()),(0,0,(),(),()),(171412,190,(6230,0,0),(7103,6716,6650,6649,1487),()),(183015,213,(),(7188,6652,7194,1485,6646),()),(178701,184,(),(6807,6652,1498,6646),()),(175856,184,(),(6782,1498,6646),()),(178807,184,(),(6807,43,7193,1498,6646),()),(181661,177,(),(6652,1472,5874,6616),()),(178933,184,(6170,0,0),(6807,6652,7194,1498,6646),()),(178781,184,(),(6807,6652,7194,1498,6646),()),(184052,184,(),(6789,1498,6646),()),(175884,184,(),(6782,1498,6646),()),(180123,184,(6208,0,0),(6807,6652,1498,6646),()),(178866,184,(6229,0,0),(6807,6652,1498,6646),()),(0,0,(),(),()),(0,0,(),(),())],[Player-3685-0A49ECD5,21562,Player-3685-0CF2175A,267344],9,0,0,0"#,
                output: vec![
                    "COMBATANT_INFO",
                    "Player-3685-0CF2175A",
                    "0",
                    "1042",
                    "155",
                    "1502",
                    "448",
                    "0",
                    "0",
                    "0",
                    "307",
                    "307",
                    "307",
                    "0",
                    "0",
                    "518",
                    "518",
                    "518",
                    "0",
                    "128",
                    "495",
                    "495",
                    "495",
                    "1038",
                    "70",
                    "(269569,326732,234299,230332,223817,326734,317866)",
                    "(0,210256,246806,305394)",
                    "[7,1,[],[(1305),(1312),(1316),(1308),(1310)],[(216,158),(129,184)]]",
                    "[(178777,184,(),(6807,6652,7194,1498,6646),()),(175885,184,(),(6789,1498,6646),()),(175872,184,(),(6782,1498,6646),()),(0,0,(),(),()),(171412,190,(6230,0,0),(7103,6716,6650,6649,1487),()),(183015,213,(),(7188,6652,7194,1485,6646),()),(178701,184,(),(6807,6652,1498,6646),()),(175856,184,(),(6782,1498,6646),()),(178807,184,(),(6807,43,7193,1498,6646),()),(181661,177,(),(6652,1472,5874,6616),()),(178933,184,(6170,0,0),(6807,6652,7194,1498,6646),()),(178781,184,(),(6807,6652,7194,1498,6646),()),(184052,184,(),(6789,1498,6646),()),(175884,184,(),(6782,1498,6646),()),(180123,184,(6208,0,0),(6807,6652,1498,6646),()),(178866,184,(6229,0,0),(6807,6652,1498,6646),()),(0,0,(),(),()),(0,0,(),(),())]",
                    "[Player-3685-0A49ECD5,21562,Player-3685-0CF2175A,267344]",
                    "9",
                    "0",
                    "0",
                    "0"
                ]
            }
        ];

        for td in &test_data {
            let tokens = split_wow_combat_log_tokens(td.input);
            assert_eq!(tokens, td.output);
        }
    }
}