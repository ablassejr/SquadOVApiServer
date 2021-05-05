use crate::{
    csgo::demo::{
        CsgoDemo,
        CsgoDemoBombStatus,
        CsgoDemoBombSite,
        CsgoTeam,
        CsgoRoundWin,
        CsgoDemoHitGroup,
    },
    csgo::weapon::{
        CsgoWeapon,
        csgo_string_to_weapon,
    },
    csgo::gsi::CsgoGsiMatchState,
    steam::SteamAccount,
    SquadOvError,
};
use serde::{Serialize};
use serde_repr::{Serialize_repr};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;

pub struct CsgoView {
    pub view_uuid: Uuid,
    pub match_uuid: Option<Uuid>,
    pub user_id: i64,
    pub has_gsi: bool,
    pub has_demo: bool,
    pub map: String,
    pub mode: String,
    pub game_server: String,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Serialize_repr, Debug, TryFromPrimitive, PartialEq)]
#[repr(i32)]
pub enum CsgoEventSource {
    Gsi,
    Demo
}

#[derive(Serialize)]
pub struct CsgoCommonPlayer {
    pub container_id: i64,
    pub user_id: i32,
    pub steam_account: SteamAccount,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub mvps: i32,
}

#[derive(Serialize)]
pub struct CsgoCommonRoundPlayerStats {
    pub container_id: i64,
    pub round_num: i32,
    pub user_id: i32,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub mvp: bool,
    pub equipment_value: Option<i32>,
    pub money: Option<i32>,
    pub headshot_kills: Option<i32>,
    pub utility_damage: Option<i32>,
    pub enemies_flashed: Option<i32>,
    pub damage: Option<i32>,
    pub armor: Option<i32>,
    pub has_defuse: Option<bool>,
    pub has_helmet: Option<bool>,
    pub team: CsgoTeam,
    pub weapons: Vec<CsgoWeapon>,
}

#[derive(Serialize)]
pub struct CsgoCommonRoundKill {
    pub container_id: i64,
    pub round_num: i32,
    pub tm: DateTime<Utc>,
    pub victim: Option<i32>,
    pub killer: Option<i32>,
    pub assister: Option<i32>,
    pub flash_assist: Option<bool>,
    pub headshot: Option<bool>,
    pub smoke: Option<bool>,
    pub blind: Option<bool>,
    pub wallbang: Option<bool>,
    pub noscope: Option<bool>,
    pub weapon: Option<CsgoWeapon>,
}

#[derive(Serialize)]
pub struct CsgoCommonRoundDamage {
    pub container_id: i64,
    pub round_num: i32,
    pub tm: DateTime<Utc>,
    pub receiver: i32,
    pub attacker: Option<i32>,
    pub remaining_health: i32,
    pub remaining_armor: i32,
    pub damage_health: i32,
    pub damage_armor: i32,
    pub weapon: CsgoWeapon,
    pub hitgroup: CsgoDemoHitGroup,
}

#[derive(Serialize)]
pub struct CsgoCommonRound {
    pub container_id: i64,
    pub round_num: i32,
    pub tm_round_start: Option<DateTime<Utc>>,
    pub tm_round_play: Option<DateTime<Utc>>,
    pub tm_round_end: Option<DateTime<Utc>>,
    pub bomb_state: Option<CsgoDemoBombStatus>,
    pub tm_bomb_plant: Option<DateTime<Utc>>,
    pub bomb_plant_user: Option<i32>,
    pub bomb_plant_site: Option<CsgoDemoBombSite>,
    pub tm_bomb_event: Option<DateTime<Utc>>,
    pub bomb_event_user: Option<i32>,
    pub winning_team: Option<CsgoTeam>,
    pub round_win_reason: Option<CsgoRoundWin>,
    pub round_mvp: Option<i32>,
    pub player_stats: Vec<CsgoCommonRoundPlayerStats>,
    pub kills: Vec<CsgoCommonRoundKill>,
    pub damage: Vec<CsgoCommonRoundDamage>,
}

#[derive(Serialize)]
pub struct CsgoCommonEventContainer {
    pub id: i64,
    pub view_uuid: Uuid,
    pub event_source: CsgoEventSource,
    pub rounds: Vec<CsgoCommonRound>,
    pub players: Vec<CsgoCommonPlayer>,
}

fn csgo_gsi_team_to_common(team: &str) -> CsgoTeam {
    match team {
        "CT" => CsgoTeam::TeamCT,
        "T" => CsgoTeam::TeamT,
        _ => CsgoTeam::TeamSpectate
    }
}

impl CsgoCommonEventContainer {
    pub fn from_demo(demo: &CsgoDemo, ref_timestamp: &DateTime<Utc>) -> Result<Self, SquadOvError> {
        let mut ret = Self{
            // These two are just temporary
            id: 0,
            view_uuid: Uuid::new_v4(),
            event_source: CsgoEventSource::Demo,
            rounds: vec![],
            players: vec![],
        };

        let tick_rate = demo.header.playback_ticks as f32 / demo.header.playback_time;
        let tick_to_timestamp = |tick| {
            ref_timestamp.clone() + chrono::Duration::milliseconds((tick as f32 / tick_rate * 1000.0) as i64)
        };

        let mut match_players: HashMap<i32, CsgoCommonPlayer> = HashMap::new();
        for (user_id, player) in &demo.player_info {
            match_players.insert(*user_id, CsgoCommonPlayer{
                container_id: 0,
                user_id: *user_id,
                steam_account: SteamAccount{
                    steam_id: player.xuid as i64,
                    name: player.name.clone(),
                },
                kills: 0,
                deaths: 0,
                assists: 0,
                mvps: 0,
            });
        }

        for round in &demo.rounds {
            let mut new_round = CsgoCommonRound{
                container_id: 0,
                round_num: round.round_num as i32,
                tm_round_start: Some(tick_to_timestamp(round.round_start_tick)),
                tm_round_play: round.freeze_end_tick.map(tick_to_timestamp),
                tm_round_end: round.round_end_tick.map(tick_to_timestamp),
                bomb_state: None,
                tm_bomb_plant: None,
                bomb_plant_site: None,
                bomb_plant_user: None,
                tm_bomb_event: None,
                bomb_event_user: None,
                winning_team: round.round_winner.clone(),
                round_win_reason: round.round_win_reason.clone(),
                round_mvp: round.round_mvp.clone(),
                player_stats: vec![],
                kills: vec![],
                damage: vec![],
            };

            if let Some(bomb_state) = &round.bomb_state {
                new_round.bomb_state = Some(bomb_state.bomb_state);
                new_round.tm_bomb_plant = bomb_state.bomb_event_tick.map(tick_to_timestamp);
                new_round.bomb_plant_site = bomb_state.bomb_plant_site.clone();
                new_round.bomb_plant_user = bomb_state.bomb_plant_userid.clone();
                new_round.tm_bomb_event = bomb_state.bomb_event_tick.map(tick_to_timestamp);
                new_round.bomb_event_user = bomb_state.bomb_event_userid.clone();
            }

            for k in &round.kills {
                let new_kill = CsgoCommonRoundKill{
                    container_id: 0,
                    round_num: round.round_num as i32,
                    tm: tick_to_timestamp(k.tick),
                    victim: Some(k.victim),
                    killer: k.killer.clone(),
                    assister: k.assister.clone(),
                    flash_assist: Some(k.flash_assist),
                    headshot: Some(k.headshot),
                    smoke: Some(k.smoke),
                    blind: Some(k.blind),
                    wallbang: Some(k.wallbang),
                    noscope: Some(k.noscope),
                    weapon: Some(k.weapon),
                };
                new_round.kills.push(new_kill);
            }

            for d in &round.damage {
                let new_damage = CsgoCommonRoundDamage{
                    container_id: 0,
                    round_num: round.round_num as i32,
                    tm: tick_to_timestamp(d.tick),
                    receiver: d.receiver,
                    attacker: d.attacker,
                    remaining_health: d.remaining_health,
                    remaining_armor: d.remaining_armor,
                    damage_health: d.damage_health,
                    damage_armor: d.damage_armor,
                    weapon: d.weapon,
                    hitgroup: d.hitgroup,
                };
                new_round.damage.push(new_damage);
            }

            for (user_id, p) in &round.players {
                let is_mvp = if let Some(mvp_id) = round.round_mvp {
                    mvp_id == *user_id
                } else {
                    false
                };

                if let Some(match_player) = match_players.get_mut(user_id) {
                    match_player.kills += p.kills;
                    match_player.deaths += p.deaths;
                    match_player.assists += p.assists;
                    
                    if is_mvp {
                        match_player.mvps += 1;
                    }
                }

                let new_stats = CsgoCommonRoundPlayerStats{
                    container_id: 0,
                    round_num: round.round_num as i32,
                    user_id: *user_id,
                    kills: p.kills,
                    deaths: p.deaths,
                    assists: p.assists,
                    mvp: is_mvp,
                    equipment_value: Some(p.equipment_value),
                    money: Some(p.money),
                    headshot_kills: Some(p.headshot_kills),
                    utility_damage: Some(p.utility_damage),
                    enemies_flashed: Some(p.enemies_flashed),
                    damage: Some(p.damage),
                    armor: Some(p.armor),
                    has_defuse: Some(p.has_defuse),
                    has_helmet: Some(p.has_helmet),
                    team: p.team,
                    weapons: p.weapons.clone(),
                };
                new_round.player_stats.push(new_stats);
            }

            ret.rounds.push(new_round);
        }

        ret.players = match_players.into_iter().map(|(_id, v)| { v }).collect();
        Ok(ret)
    }

    pub fn from_gsi(gsi: &CsgoGsiMatchState) -> Result<Self, SquadOvError> {
        let mut ret = Self{
            // These two are just temporary
            id: 0,
            view_uuid: Uuid::new_v4(),
            event_source: CsgoEventSource::Gsi,
            rounds: vec![],
            players: vec![],
        };

        let mut steam_id_to_user_id: HashMap<i64, i32> = HashMap::new();
        {
            // The players hash map probably only has 1 player in it but we iterate through it to be sure.
            // It's indexed by the user's Steam ID in string form.
            let mut next_user_id: i32 = 1;
            let mut player_map: HashMap<i64, CsgoCommonPlayer> = HashMap::new();
            for (steamid, gsi_player) in &gsi.players {
                let steamid = steamid.parse::<i64>()?;
                let new_player = CsgoCommonPlayer{
                    container_id: 0,
                    user_id: next_user_id,
                    steam_account: SteamAccount{
                        steam_id: steamid,
                        name: gsi_player.name.clone(),
                    },
                    kills: gsi_player.kills,
                    deaths: gsi_player.deaths,
                    assists: gsi_player.assists,
                    mvps: gsi_player.mvps,
                };

                steam_id_to_user_id.insert(steamid, new_player.user_id);
                player_map.insert(steamid, new_player);
                next_user_id += 1;
            }
            ret.players = player_map.into_iter().map(|(_id, v)| { v }).collect();
        }

        {
            let mut round_map: HashMap<i32, CsgoCommonRound> = HashMap::new();
            for (round_num, round) in &gsi.rounds {
                let round_win_reason: CsgoRoundWin = CsgoRoundWin::TWin;
                let mut new_round = CsgoCommonRound{
                    container_id: 0,
                    round_num: *round_num,
                    tm_round_start: round.buy_time.clone(),
                    tm_round_play: round.play_time.clone(),
                    tm_round_end: round.round_end_time.clone(),
                    bomb_state: if round.bomb_next_time.is_some() {
                        Some(match round_win_reason {
                            CsgoRoundWin::BombDefused => CsgoDemoBombStatus::Defused,
                            _ => CsgoDemoBombStatus::Exploded,
                        })
                    } else if round.bomb_plant_time.is_some() {
                        Some(CsgoDemoBombStatus::Planted)
                    } else {
                        None
                    },
                    tm_bomb_plant: round.bomb_plant_time.clone(),
                    bomb_plant_site: None,
                    bomb_plant_user: None,
                    tm_bomb_event: round.bomb_next_time.clone(),
                    bomb_event_user: None,
                    winning_team: round.winning_team.as_ref().map(|x| { csgo_gsi_team_to_common(x) }),
                    round_win_reason: Some(round_win_reason),
                    round_mvp: None,
                    player_stats: vec![],
                    kills: vec![],
                    damage: vec![],
                };

                let mut round_player_stats: HashMap<i64, CsgoCommonRoundPlayerStats> = HashMap::new();
                for (steamid, p) in &round.players {
                    let steamid = steamid.parse::<i64>()?;
                    if let Some(userid) = steam_id_to_user_id.get(&steamid) {
                        let pround = CsgoCommonRoundPlayerStats{
                            container_id: 0,
                            round_num: *round_num,
                            user_id: *userid,
                            kills: 0,
                            deaths: 0,
                            assists: 0,
                            mvp: false,
                            equipment_value: Some(p.equipment_value),
                            money: Some(p.money),
                            headshot_kills: None,
                            utility_damage: None,
                            enemies_flashed: None,
                            damage: None,
                            armor: Some(p.armor),
                            has_defuse: None,
                            has_helmet: Some(p.helmet),
                            team: csgo_gsi_team_to_common(&p.team),
                            weapons: p.weapons.values().map(|x| {
                                csgo_string_to_weapon(&x.name)
                            }).collect(),
                        };
                        round_player_stats.insert(steamid, pround);
                    }
                }
                
                for k in &round.kills {
                    let mut base_kill = CsgoCommonRoundKill{
                        container_id: 0,
                        round_num: *round_num,
                        tm: k.timestamp.clone(),
                        victim: None,
                        killer: None,
                        assister: None,
                        flash_assist: None,
                        headshot: k.headshot.clone(),
                        smoke: k.smoked.clone(),
                        blind: k.flashed.clone(),
                        wallbang: None,
                        noscope: None,
                        weapon: k.weapon.as_ref().map(|x| {
                            csgo_string_to_weapon(x)
                        }),
                    };

                    if let Some(killer) = &k.killer {
                        let steamid = killer.parse::<i64>()?;
                        if let Some(stats) = round_player_stats.get_mut(&steamid) {
                            stats.kills += 1;
                        }
                        base_kill.killer = steam_id_to_user_id.get(&steamid).cloned();
                    } else if let Some(victim) = &k.victim {
                        let steamid = victim.parse::<i64>()?;
                        if let Some(stats) = round_player_stats.get_mut(&steamid) {
                            stats.deaths += 1;
                        }
                        base_kill.victim = steam_id_to_user_id.get(&steamid).cloned();
                    } else if let Some(assister) = k.assisters.first() {
                        let steamid = assister.parse::<i64>()?;
                        if let Some(stats) = round_player_stats.get_mut(&steamid) {
                            stats.assists += 1;
                        }
                        base_kill.assister = steam_id_to_user_id.get(&steamid).cloned();
                    } else {
                        continue;
                    }

                    new_round.kills.push(base_kill);
                }

                new_round.player_stats = round_player_stats.into_iter().map(|(_id, v)| { v }).collect();
                round_map.insert(*round_num, new_round);
            }
            ret.rounds = round_map.into_iter().map(|(_id, v)| { v }).collect();
        }

        Ok(ret)
    }
}