use serde::Deserialize;
use chrono::{DateTime, Utc};
use std::convert::TryInto;
use std::cmp::PartialEq;
use std::str::FromStr;
use crate::{
    SquadOvError,
    combatlog::{CombatLogPacket},
};
use unicode_segmentation::UnicodeSegmentation;
use serde::Serialize;

#[derive(Deserialize, Clone)]
#[serde(rename_all="camelCase")]
pub struct WoWCombatLogState {
    pub combat_log_version: String,
    pub advanced_log: bool,
    pub build_version: String
}

pub struct FullWoWCombatLogState {
    pub state: WoWCombatLogState,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct RawWoWCombatLogPayload {
    pub timestamp: DateTime<Utc>,
    pub parts: Vec<String>,
    pub raw_log: String,
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
            &self.raw_log,
        )
    }

    pub fn redo_parts(&mut self) {
        // I'm not really sure if we need to do unicode aware stepping here
        // but it's probably better safe than sorry.
        self.parts = split_wow_combat_log_tokens(&self.raw_log);
    }

    pub fn is_finish_token(&self) -> bool {
        self.parts.len() > 0 && self.parts[0] == "SQUADOV_END_COMBAT_LOG"
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WoWSpellInfo {
    pub id: i64,
    name: String,
    pub school: i64
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WowCovenantInfo {
    pub covenant_id: i32,
    pub soulbind_id: i32,
    pub soulbind_traits: Vec<i32>,
    pub conduits: Vec<WoWItemInfo>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="type")]
pub enum WoWDamageType {
    SwingDamage,
    SpellDamage(WoWSpellInfo)
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
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

impl ToString for WoWSpellAuraType {
    fn to_string(&self) -> String {
        String::from(match self {
            WoWSpellAuraType::Buff => "BUFF",
            WoWSpellAuraType::Debuff => "DEBUFF",
            WoWSpellAuraType::Unknown => "UNKNOWN"
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WoWItemInfo {
    pub item_id: i64,
    pub ilvl: i32
}

fn parse_wow_item_info_from_str(s: &str) -> Result<Vec<WoWItemInfo>, SquadOvError> {
    if s.len() < 2 {
        return Ok(vec![]);
    }

    let tokens = split_wow_combat_log_tokens(&s[1..s.len()-1]);
    // Each top level token is for one given item.
    Ok(tokens.into_iter().map(|x| {
        if x.len() < 2 {
            return WoWItemInfo {
                item_id: -1,
                ilvl: -1,
            }
        }

        let item_parts = split_wow_combat_log_tokens(&x[1..x.len()-1]);
        WoWItemInfo{
            item_id: item_parts[0].parse().unwrap_or(-1),
            ilvl: item_parts[1].parse().unwrap_or(-1),
        }
    }).collect())
}

fn parse_wow_talents_from_str(s: &str) -> Result<Vec<i32>, SquadOvError> {
    if s.len() < 2 {
        return Ok(vec![]);
    }
    
    let tokens = split_wow_combat_log_tokens(&s[1..s.len()-1]);
    Ok(tokens.into_iter().map(|x| {
        Ok(x.parse()?)
    }).collect::<Result<Vec<i32>, SquadOvError>>()?)
}

fn parse_soulbind_traits_from_str(s: &str) -> Result<Vec<i32>, SquadOvError> {
    if s.len() < 2 {
        return Ok(vec![]);
    }

    let tokens = split_wow_combat_log_tokens(&s[1..s.len()-1]);
    Ok(tokens.into_iter().map(|x| {
        if x.len() < 2 {
            return Ok(0);
        }
        let inner = split_wow_combat_log_tokens(&x[1..x.len()-1]);
        Ok(inner[0].parse()?)
    }).collect::<Result<Vec<i32>, SquadOvError>>()?)
}

fn parse_wow_covenant_from_str(s: &str) -> Result<Option<WowCovenantInfo>, SquadOvError> {
    if s.len() < 2 {
        return Ok(None);
    }

    let tokens = split_wow_combat_log_tokens(&s[1..s.len()-1]);

    if tokens.len() < 4 {
        return Ok(None);
    }

    Ok(
        Some(
            WowCovenantInfo{
                covenant_id: tokens[1].parse()?,
                soulbind_id: tokens[0].parse()?,
                soulbind_traits: parse_soulbind_traits_from_str(&tokens[3])?,
                conduits: parse_wow_item_info_from_str(&tokens[4])?
            }
        )
    )
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WowClassUpdateFromSpell {
    player_guid: String,
    spell_id: i64,
    // The latter two are necessary if we want to update the user character cache too.
    // We will only do that in the case where user_id is not None.
    player_name: String,
    user_id: Option<i64>,
    build_version: String,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="type")]
pub enum WoWCombatLogEventType {
    UnitDied{
        unconcious: bool,
    },
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
    AuraBreak{
        aura: WoWSpellInfo,
        spell: Option<WoWSpellInfo>,
        aura_type: WoWSpellAuraType,
    },
    SpellCast{
        spell: WoWSpellInfo,
        start: bool,
        finish: bool,
        success: bool,
    },
    SpellSummon(WoWSpellInfo),
    CombatantInfo{
        guid: String,
        team: i32,
        strength: i32,
        agility: i32,
        stamina: i32,
        intelligence: i32,
        armor: i32,
        spec_id: i32,
        talents: Vec<i32>,
        pvp_talents: Vec<i32>,
        covenant: Option<WowCovenantInfo>,
        items: Vec<WoWItemInfo>,
        rating: i32,
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
    ArenaStart{
        instance_id: i32,
        arena_type: String,
        local_team_id: i32,
    },
    ArenaEnd{
        winning_team_id: i32,
        match_duration_seconds: i32,
        new_ratings: Vec<i32>,
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
    ClassUpdateFromSpell(WowClassUpdateFromSpell),
    Unknown
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WoWCombatLogSourceDest {
    pub guid: String,
    pub name: String,
    pub flags: i64,
    raid_flags: i64,
}

impl WoWCombatLogSourceDest {
    fn new(data: &[String; 4]) -> Result<Self, crate::SquadOvError> {
        if !data[2].starts_with("0x") || !data[3].starts_with("0x") {
            return Err(crate::SquadOvError::BadRequest);
        }

        Ok(Self {
            guid: data[0].clone(),
            name: data[1].clone(),
            flags: i64::from_str_radix(&data[2][2..], 16)?,
            raid_flags: i64::from_str_radix(&data[3][2..], 16)?,
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="type")]
pub enum WoWGenericLevel {
    ItemLevel{
        level: i32
    },
    UnitLevel{
        level: i32
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="type")]
pub struct WoWCombatLogAdvancedCVars {
    pub unit_guid: String,
    pub owner_guid: String,
    current_hp: i64,
    max_hp: i64,
    attack_power: i64,
    spell_power: i64,
    armor: i64,
    unk1: i64,
    resource_type: Vec<i32>,
    current_resource: Vec<i64>,
    max_resource: Vec<i64>,
    resource_cost: Vec<i64>,
    coord0: f64,
    coord1: f64,
    map_id: i64,
    facing: f64,
    level: WoWGenericLevel
}

impl WoWCombatLogAdvancedCVars {
    fn new(data: &[String], use_ilvl: bool) -> Result<Self, crate::SquadOvError> {
        let has_armor = data.len() == 17;

        let level: i32 = data[if has_armor { 16 } else { 15 }].parse()?;
        let resource_type: Vec<i32> = data[if has_armor { 8 } else { 7 }].split('|').map(|x| {
            Ok(x.parse()?)
        }).collect::<Result<Vec<i32>, SquadOvError>>()?;
        let current_resource: Vec<i64> = data[if has_armor { 9 } else { 8 }].split('|').map(|x| {
            Ok(x.parse()?)
        }).collect::<Result<Vec<i64>, SquadOvError>>()?;
        let max_resource: Vec<i64> = data[if has_armor { 10 } else { 9 }].split('|').map(|x| {
            Ok(x.parse()?)
        }).collect::<Result<Vec<i64>, SquadOvError>>()?;
        let resource_cost: Vec<i64> = data[if has_armor { 11 } else { 10 }].split('|').map(|x| {
            Ok(x.parse()?)
        }).collect::<Result<Vec<i64>, SquadOvError>>()?;

        Ok(Self {
            unit_guid: data[0].clone(),
            owner_guid: data[1].clone(),
            current_hp: data[2].parse()?,
            max_hp: data[3].parse()?,
            attack_power: data[4].parse()?,
            spell_power: data[5].parse()?,
            armor: if has_armor { data[6].parse()? } else { 0 },
            unk1: data[if has_armor { 7 } else { 6}].parse()?,
            resource_type,
            current_resource,
            max_resource,
            resource_cost,
            coord0: data[if has_armor { 12 } else { 11} ].parse()?,
            coord1: data[if has_armor { 13 } else { 12} ].parse()?,
            map_id: data[if has_armor { 14 } else { 13 }].parse()?,
            facing: data[if has_armor { 15 } else { 14} ].parse()?,
            level: if use_ilvl {
                WoWGenericLevel::ItemLevel{level}
            } else {
                WoWGenericLevel::UnitLevel{level}
            }
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WoWCombatLogEvent {
    pub timestamp: DateTime<Utc>,
    pub source: Option<WoWCombatLogSourceDest>,
    pub dest: Option<WoWCombatLogSourceDest>,
    pub advanced: Option<WoWCombatLogAdvancedCVars>,
    pub event: WoWCombatLogEventType
}


#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="form")]
pub enum WowPacketData {
    Raw{inner: String},
    Parsed{inner: WoWCombatLogEvent},
    Flush,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct WowCombatLogPacket {
    pub partition_id: String,
    pub time: DateTime<Utc>,
    pub data: WowPacketData,
}

pub fn parse_advanced_cvars_and_event_from_wow_combat_log(state: &WoWCombatLogState, payload: &RawWoWCombatLogPayload) -> Result<(Option<WoWCombatLogAdvancedCVars>, WoWCombatLogEventType), crate::SquadOvError> {
    let action_parts: Vec<&str> = payload.parts[0].split("_").collect();
    let cl_version: i32 = state.combat_log_version.parse::<i32>()?;
    let advanced_cvar_offset: usize = if cl_version == 9 { 16 } else { 17 };

    match action_parts[0] {
        "SPELL" => {
            match action_parts[1] {
                // Need to handle ABSORBED separately since its format is different from other spells.
                "ABSORBED" => Ok((None, WoWCombatLogEventType::Unknown)),
                _ => {
                    if payload.parts.len() < 12 {
                        return Ok((None, WoWCombatLogEventType::Unknown));
                    }

                    let mut idx = 9;

                    if !payload.parts[idx+2].starts_with("0x") {
                        log::warn!("Invalid spell school: {}", payload.flatten());
                        return Ok((None, WoWCombatLogEventType::Unknown));
                    }

                    let spell_info = WoWSpellInfo{
                        id: payload.parts[idx].parse()?,
                        name: payload.parts[idx+1].clone(),
                        school: i64::from_str_radix(&payload.parts[idx+2][2..], 16)?,
                    };
                    idx += 3;
                    
                    match payload.parts[0].as_str() {
                        "SPELL_DAMAGE" | "SPELL_PERIODIC_DAMAGE" => Ok({
                            let advanced = if state.advanced_log && (payload.parts.len() >= (idx+advanced_cvar_offset)) {
                                Some(WoWCombatLogAdvancedCVars::new(&payload.parts[idx..idx+advanced_cvar_offset], payload.parts[0].as_str() == "SPELL_DAMAGE")?)
                            } else {
                                None
                            };
                            idx += advanced_cvar_offset;
        
                            (
                                advanced,
                                if payload.parts.len() >= (idx+2) { 
                                    WoWCombatLogEventType::DamageDone{
                                        damage: WoWDamageType::SpellDamage(spell_info),
                                        amount: payload.parts[idx].parse()?,
                                        overkill: payload.parts[idx+1].parse()?,    
                                    }
                                } else {
                                    WoWCombatLogEventType::Unknown
                                },
                            )
                        }),
                        "SPELL_HEAL" | "SPELL_PERIODIC_HEAL" => Ok({
                            let advanced = if state.advanced_log && (payload.parts.len() >= (idx+advanced_cvar_offset)) {
                                Some(WoWCombatLogAdvancedCVars::new(&payload.parts[idx..idx+advanced_cvar_offset], true)?)
                            } else {
                                None
                            };
                            idx += advanced_cvar_offset;
        
                            (
                                advanced,
                                if payload.parts.len() >= (idx+4) { 
                                    WoWCombatLogEventType::Healing{
                                        spell: spell_info,
                                        amount: payload.parts[idx+1].parse()?,
                                        overheal: payload.parts[idx+2].parse()?,
                                        absorbed: payload.parts[idx+3].parse()?,
                                    }
                                } else {
                                    WoWCombatLogEventType::Unknown
                                },
                            )
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
                        "SPELL_AURA_BROKEN_SPELL" => Ok({
                            let by_spell = WoWSpellInfo{
                                id: payload.parts[idx].parse()?,
                                name: payload.parts[idx+1].clone(),
                                // Not sure why but this one is decimal instead of hex.
                                school: i64::from_str_radix(&payload.parts[idx+2][..], 10)?,
                            };
                            idx += 3;

                            (None, WoWCombatLogEventType::AuraBreak{
                                aura: spell_info,
                                spell: Some(by_spell),
                                aura_type: WoWSpellAuraType::from_str(&payload.parts[idx])?,
                            })
                        }),
                        "SPELL_AURA_BROKEN" => Ok((None, WoWCombatLogEventType::AuraBreak{
                            aura: spell_info,
                            spell: None,
                            aura_type: WoWSpellAuraType::from_str(&payload.parts[idx])?,
                        })),
                        "SPELL_CAST_START" => Ok((None, WoWCombatLogEventType::SpellCast{
                            spell: spell_info,
                            start: true,
                            finish: false,
                            success: false,
                        })),
                        "SPELL_CAST_SUCCESS" => Ok({
                            let advanced = if state.advanced_log && (payload.parts.len() >= (idx+advanced_cvar_offset)) {
                                Some(WoWCombatLogAdvancedCVars::new(&payload.parts[idx..idx+advanced_cvar_offset], true)?)
                            } else {
                                None
                            };
                            
                            (
                                advanced,
                                WoWCombatLogEventType::SpellCast{
                                    spell: spell_info,
                                    start: false,
                                    finish: true,
                                    success: true,
                                },
                            )
                        }),
                        "SPELL_CAST_FAILED" => Ok((None, WoWCombatLogEventType::SpellCast{
                            spell: spell_info,
                            start: false,
                            finish: true,
                            success: false,
                        })),
                        _ => Ok((None, WoWCombatLogEventType::Unknown)),
                    }
                }
            }
        },
        _ => match payload.parts[0].as_str() {
            "UNIT_DIED" => Ok((None, WoWCombatLogEventType::UnitDied{
                unconcious: if cl_version == 9 { false } else { payload.parts[9] == "1" },
            })),
            "SWING_DAMAGE_LANDED" => if payload.parts.len() >= 9 {
                Ok({
                    let mut idx = 9;
                    let advanced = if state.advanced_log && (payload.parts.len() >= (idx+advanced_cvar_offset)) {
                        Some(WoWCombatLogAdvancedCVars::new(&payload.parts[idx..idx+advanced_cvar_offset], false)?)
                    } else {
                        None
                    };
                    idx += advanced_cvar_offset;

                    (advanced, WoWCombatLogEventType::DamageDone{
                        damage: WoWDamageType::SwingDamage,
                        amount: payload.parts[idx].parse()?,
                        overkill: payload.parts[idx+1].parse()?,
                    })
                })
            } else {
                log::warn!("Bad SWING_DAMAGE_LANDED line: {} - {} @ {}", payload.flatten(), payload.parts.len(), cl_version);
                Ok((None, WoWCombatLogEventType::Unknown))
            },
            "COMBATANT_INFO" =>  if cl_version == 9 && payload.parts.len() >= 31 {
                Ok((None, WoWCombatLogEventType::CombatantInfo{
                    guid: payload.parts[1].clone(),
                    team: payload.parts[2].parse()?,
                    strength: payload.parts[3].parse()?,
                    agility: payload.parts[4].parse()?,
                    stamina: payload.parts[5].parse()?,
                    intelligence: payload.parts[6].parse()?,
                    armor: 0,
                    spec_id: 0,
                    talents: vec![],
                    pvp_talents: vec![],
                    covenant: None,
                    items: parse_wow_item_info_from_str(&payload.parts[27])?,
                    rating: payload.parts[30].parse()?,
                }))
            } else if payload.parts.len() >= 33 {
                Ok((None, WoWCombatLogEventType::CombatantInfo{
                    guid: payload.parts[1].clone(),
                    team: payload.parts[2].parse()?,
                    strength: payload.parts[3].parse()?,
                    agility: payload.parts[4].parse()?,
                    stamina: payload.parts[5].parse()?,
                    intelligence: payload.parts[6].parse()?,
                    armor: payload.parts[23].parse()?,
                    spec_id: payload.parts[24].parse()?,
                    talents: parse_wow_talents_from_str(&payload.parts[25])?,
                    pvp_talents: parse_wow_talents_from_str(&payload.parts[26])?,
                    covenant: parse_wow_covenant_from_str(&payload.parts[27])?,
                    items: parse_wow_item_info_from_str(&payload.parts[28])?,
                    rating: payload.parts[32].parse()?,
                }))
            } else {
                log::warn!("Bad COMBATANT_INFO line: {} - {} @ {}", payload.flatten(), payload.parts.len(), cl_version);
                Ok((None, WoWCombatLogEventType::Unknown))
            },
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
            "ARENA_MATCH_START" => Ok((None, WoWCombatLogEventType::ArenaStart{
                instance_id: payload.parts[1].parse()?,
                arena_type: payload.parts[3].clone(),
                local_team_id: payload.parts[4].parse()?,
            })),
            "ARENA_MATCH_END" => Ok((None, WoWCombatLogEventType::ArenaEnd{
                winning_team_id: payload.parts[1].parse()?,
                match_duration_seconds: payload.parts[2].parse()?,
                new_ratings: vec![
                    payload.parts[3].parse()?,
                    payload.parts[4].parse()?,
                ],
            })),
            _ => Ok((None, WoWCombatLogEventType::Unknown))
        },
    }
}

pub fn parse_raw_wow_combat_log_payload(state: &WoWCombatLogState, payload: &RawWoWCombatLogPayload) -> Result<Option<WoWCombatLogEvent>, crate::SquadOvError> {    
    let (advanced, event) = std::panic::catch_unwind(|| {
        parse_advanced_cvars_and_event_from_wow_combat_log(state, payload).unwrap_or((None, WoWCombatLogEventType::Unknown))
    }).map_err(|e| {
        crate::SquadOvError::InternalError(
            format!("Unable to Parse WoW Combat Log Line: {:?} - Payload {:?}", e, payload)
        )
    })?;

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
            "ARENA_MATCH_START" |
            "ARENA_MATCH_END" |
            "COMBATANT_INFO" => false,
        _ => true,
    };

    Ok(Some(
        WoWCombatLogEvent{
            timestamp: payload.timestamp.clone(),
            source: if has_source_dest {
                if let Ok(x) = WoWCombatLogSourceDest::new(payload.parts[1..5].try_into()?) {
                    Some(x)
                } else {
                    log::warn!("Failed to parse source for WoW combat log payload: {}", payload.flatten());
                    None
                }
            } else {
                None
            },
            dest: if has_source_dest {
                if let Ok(x) = WoWCombatLogSourceDest::new(payload.parts[5..9].try_into()?) {
                    Some(x)
                } else {
                    log::warn!("Failed to parse dest for WoW combat log payload: {}", payload.flatten());
                    None
                }
            } else {
                None
            },
            advanced,
            event
        }
    ))
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
            raw_log: None,
            log_line: 1,
            version: 2,
        };
        assert_eq!(t1.is_finish_token(), false);

        let t2 = RawWoWCombatLogPayload{
            timestamp: Utc::now(),
            parts: vec![String::from("SQUADOV_END_COMBAT_LOG")],
            raw_log: None,
            log_line: 1,
            version: 2,
        };
        assert_eq!(t2.is_finish_token(), true);
    }

    #[test]
    fn test_split_wow_combat_log_tokens() {
        init();

        struct TestDatum {
            input: &'static str,
            output: Vec<&'static str>,
        }

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
            },
            TestDatum{
                input: r#"ENCOUNTER_START,2382,"Amarth, The Harvester",23,5,2289"#,
                output: vec![
                    "ENCOUNTER_START",
                    "2382",
                    "Amarth, The Harvester",
                    "23",
                    "5",
                    "2289"
                ]
            }
        ];

        for td in &test_data {
            let tokens = split_wow_combat_log_tokens(td.input);
            assert_eq!(tokens, td.output);
        }
    }
}

impl CombatLogPacket for WowCombatLogPacket {
    type Data = WowCombatLogPacket;

    fn parse_from_raw(partition_key: String, raw: String, cl_state: serde_json::Value) -> Result<Option<Self::Data>, SquadOvError> {
        let cl_state = serde_json::from_value::<WoWCombatLogState>(cl_state)?;
        let mut payload = serde_json::from_str::<RawWoWCombatLogPayload>(&raw)?;
        payload.redo_parts();

        let parsed = parse_raw_wow_combat_log_payload(
            &cl_state,
            &payload,
        );

        parsed.map(|x| {
            x.map(|y| {
                WowCombatLogPacket{
                    partition_id: partition_key,
                    time: y.timestamp.clone(),
                    data: if payload.is_finish_token() {
                        WowPacketData::Flush
                    } else {
                        WowPacketData::Parsed{inner: y}
                    },
                }
            })
        })
    }

    fn create_flush_packet(partition_key: String) -> Self::Data {
        WowCombatLogPacket{
            partition_id: partition_key,
            time: Utc::now(),
            data: WowPacketData::Flush,
        }
    }

    fn create_raw_packet(partition_key: String, tm: DateTime<Utc>, raw: String) -> Self::Data {
        WowCombatLogPacket{
            partition_id: partition_key,
            time: tm,
            data: WowPacketData::Raw{inner: raw},
        }
    }

    fn extract_timestamp(data: &Self::Data) -> DateTime<Utc> {
        data.time.clone()
    }
}