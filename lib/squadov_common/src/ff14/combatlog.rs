use crate::SquadOvError;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="type")]
pub enum Ff14CombatLogEvent {
    // 0
    LogLine,
    // 1
    ChangeZone{
        id: i64,
        name: String,
    },
    // 2
    ChangePrimaryPlayer{
        id: i64,
        name: String,
    },
    // 3
    AddCombatant{
        id: i64,
        name: String,
        job: i64,
        level: i64,
        owner_id: Option<i64>,
        world_id: i64,
        world: String,
        npc_name_id: Option<i64>,
        npc_base_id: Option<i64>,
        current_hp: i64,
        hp: i64,
        current_mp: i64,
        mp: i64,
    },
    // 4
    RemoveCombatant{
        id: i64,
    },
    // 11
    PartyList{
        count: i64,
        members: Vec<i64>,
    },
    // 12
    PlayerStats{
        job: i64,
        strength: i64,
        dexterity: i64,
        vitality: i64,
        intelligence: i64,
        mind: i64,
        piety: i64,
        attack_power: i64,
        direct_hit: i64,
        critical_hit: i64,
        attack_magic_potency: i64,
        heal_magic_potency: i64,
        determination: i64,
        skill_speed: i64,
        spell_speed: i64,
        tenacity: i64,
    },
    // 20
    NetworkStartsCasting{
        source_id: i64,
        source: String,
        spell_id: i64,
        spell: String,
        target_id: i64,
        target: String,
        cast_time: f64,
    },
    // 21 and 22
    NetworkAbility{
        source_id: i64,
        source: String,
        spell_id: i64,
        spell: String,
        target_id: i64,
        target: String,
        flags: i64,
        damage: i64,
        target_current_hp: i64,
        target_hp: i64,
        target_current_mp: i64,
        target_mp: i64,
        current_hp: i64,
        hp: i64,
        current_mp: i64,
        mp: i64,
        sequence: i64,
    },
    // 23
    NetworkCancelAbility{
        source_id: i64,
        source: String,
        spell_id: i64,
        spell: String,
        reason: String,
    },
    // 24
    NetworkDot{
        id: i64,
        name: String,
        which: String, // DoT or HoT
        effect_id: i64,
        damage: i64,
        current_hp: i64,
        hp: i64,
        current_mp: i64,
        mp: i64,
    },
    // 25
    NetworkDeath{
        target_id: i64,
        target: String,
        source_id: i64,
        source: String,
    },
    // 26
    NetworkBuff{
        effect_id: i64,
        effect: String,
        duration: f64,
        source_id: i64,
        source: String,
        target_id: i64,
        target: String,
        count: i64,
        target_max_hp: i64,
        source_max_hp: i64,
    },
    // 27
    NetworkTargetIcon,
    // 28
    NetworkRaidMarker,
    // 29
    NetworkTargetMarker,
    // 30
    NetworkBuffRemove{
        effect_id: i64,
        effect: String,
        source_id: i64,
        source: String,
        target_id: i64,
        target: String,
        count: i64,
    },
    // 31
    NetworkGauge{
        id: i64,
        data0: i64,
        data1: i64,
        data2: i64,
        data3: i64,
    },
    // 32
    NetworkWorld,
    // 33
    Network6DActorControl{
        instance: i64,
        command: i64,
        data0: i64,
        data1: i64,
        data2: i64,
        data3: i64,
    },
    // 34
    NetworkNameToggle,
    // 35
    NetworkTether{
        source_id: i64,
        source: String,
        target_id: i64,
        target: String,
        tether_type: i64,
    },
    // 36
    LimitBreak{
        value: i64,
        bars: i32,
    },
    // 37
    NetworkActionSync{
        id: i64,
        name: String,
        sequence: i64,
        current_hp: i64,
        hp: i64,
        current_mp: i64,
        mp: i64,
    },
    // 38
    NetworkStatusEffects,
    // 39
    NetworkUpdateHp{
        id: i64,
        name: String,
        current_hp: i64,
        hp: i64,
        current_mp: i64,
        mp: i64,
    },
    // 40
    Map{
        region_id: i64,
        region_name: String,
        place_name: String,
        place_name_sub: String,
    },
    // 41
    SystemLogMessage,
    // 251,
    Debug,
    // 252
    PacketDump,
    // 253
    Version,
    // 254
    Error,
    Unknown,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag="form")]
pub enum Ff14PacketData {
    Raw(String),
    Parsed(Ff14CombatLogEvent),
    Flush,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct Ff14CombatLogPacket {
    pub partition_id: String,
    pub time: DateTime<Utc>,
    pub data: Ff14PacketData,
}

pub fn parse_ff14_combat_log_line(partition_id: String, line: String) -> (String, Result<Ff14CombatLogPacket, SquadOvError>) {
    (line, Err(SquadOvError::NotFound))
}