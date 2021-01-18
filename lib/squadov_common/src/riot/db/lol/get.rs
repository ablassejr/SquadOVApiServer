use crate::{
    SquadOvError,
    riot::games::{
        FullLolMatch,
        LolMatchDto,
        LolMatchTimelineDto,
        LolTeamStatsDto,
        LolTeamBansDto,
        LolParticipantDto,
        LolParticipantStatsDto,
        LolParticipantTimelineDto,
        LolParticipantIdentityDto,
        LolPlayerDto,
        LolMatchFrameDto,
        LolMatchParticipantFrameDto,
        LolMatchEventDto,
        LolMatchPositionDto,
    },
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::{BTreeSet, HashMap};

async fn get_lol_match_timeline_frames(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<LolMatchFrameDto>, SquadOvError> {
    let mut frame_map: HashMap<i64, Vec<LolMatchParticipantFrameDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT lmtpf.*
        FROM squadov.lol_match_timeline_participant_frames AS lmtpf
        WHERE lmtpf.match_uuid = $1
        ",
        match_uuid,
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !frame_map.contains_key(&x.timestamp) {
                frame_map.insert(x.timestamp, Vec::new());
            }
            let vec = frame_map.get_mut(&x.timestamp).unwrap();
            let has_position = x.x.is_some() && x.y.is_some();

            vec.push(LolMatchParticipantFrameDto{
                participant_id: x.participant_id,
                minions_killed: x.minions_killed,
                total_gold: x.total_gold,
                level: x.level,
                xp: x.xp,
                current_gold: x.current_gold,
                jungle_minions_killed: x.jungle_minions_killed,
                position: if has_position {
                    Some(LolMatchPositionDto{
                        x: x.x.unwrap(),
                        y: x.y.unwrap(),
                    })
                } else {
                    None
                }
            });
        });

    let mut event_map: HashMap<i64, Vec<LolMatchEventDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT lmte.*
        FROM squadov.lol_match_timeline_events AS lmte
        WHERE lmte.match_uuid = $1
        ",
        match_uuid,
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !event_map.contains_key(&x.timestamp) {
                event_map.insert(x.timestamp, Vec::new());
            }
            let vec = event_map.get_mut(&x.timestamp).unwrap();
            let has_position = x.x.is_some() && x.y.is_some();

            vec.push(LolMatchEventDto{
                lane_type: x.lane_type,
                skill_slot: x.skill_slot,
                ascended_type: x.ascended_type,
                creator_id: x.creator_id,
                after_id: x.after_id,
                event_type: x.event_type,
                real_type: x.real_type,
                level_up_type: x.level_up_type,
                ward_type: x.ward_type,
                participant_id: x.participant_id,
                tower_type: x.tower_type,
                item_id: x.item_id,
                before_id: x.before_id,
                monster_type: x.monster_type,
                monster_sub_type: x.monster_sub_type,
                team_id: x.team_id,
                position: if has_position {
                    Some(LolMatchPositionDto{
                        x: x.x.unwrap(),
                        y: x.y.unwrap(),
                    })
                } else {
                    None
                },
                killer_id: x.killer_id,
                timestamp: x.timestamp,
                assisting_participant_ids: x.assisting_participant_ids,
                building_type: x.building_type,
                victim_id: x.victim_id,
            });
        });

    let mut frame_timestamps: BTreeSet<i64> = BTreeSet::new();
    frame_map.iter().for_each(|(k, _)| { frame_timestamps.insert(*k); });
    event_map.iter().for_each(|(k, _)| { frame_timestamps.insert(*k); });
    
    Ok(
        frame_timestamps.into_iter().map(|timestamp| {
            LolMatchFrameDto{
                timestamp,
                participant_frames: HashMap::new(),
                events: Vec::new(),
            }
        }).collect()
    )
}

async fn get_lol_match_timeline(ex: &PgPool, match_uuid: &Uuid) -> Result<LolMatchTimelineDto, SquadOvError> {
    let data = sqlx::query!(
        "
        SELECT lmt.*
        FROM squadov.lol_match_timeline AS lmt
        WHERE lmt.match_uuid = $1
        ",
        match_uuid,
    )
        .fetch_one(&*ex)
        .await?;

    Ok(
        LolMatchTimelineDto{
            frame_interval: data.frame_interval,
            frames: get_lol_match_timeline_frames(&*ex, match_uuid).await?,
        }
    )
}

async fn get_lol_match_teams(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<LolTeamStatsDto>, SquadOvError> {
    let mut bans_per_team: HashMap<i32, Vec<LolTeamBansDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT lmb.*
        FROM squadov.lol_match_bans AS lmb
        WHERE lmb.match_uuid = $1
        ",
        match_uuid
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !bans_per_team.contains_key(&x.team_id) {
                bans_per_team.insert(x.team_id, Vec::new());
            }
            let vec = bans_per_team.get_mut(&x.team_id).unwrap();
            vec.push(LolTeamBansDto{
                champion_id: x.champion_id,
                pick_turn: x.pick_turn,
            });
        });

    Ok(
        sqlx::query!(
            "
            SELECT lmt.*
            FROM squadov.lol_match_teams AS lmt
            WHERE lmt.match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(&*ex)
            .await?
            .into_iter()
            .map(|x| {
                LolTeamStatsDto{
                    tower_kills: x.tower_kills,
                    rift_herald_kills: x.rift_herald_kills,
                    first_blood: x.first_blood,
                    inhibitor_kills: x.inhibitor_kills,
                    first_baron: x.first_baron,
                    first_dragon: x.first_dragon,
                    dragon_kills: x.dragon_kills,
                    baron_kills: x.baron_kills,
                    first_inhibitor: x.first_inhibitor,
                    first_tower: x.first_tower,
                    first_rift_herald: x.first_rift_herald,
                    team_id: x.team_id,
                    win: x.win,
                    bans: bans_per_team.remove(&x.team_id).unwrap_or(vec![]),
                }
            })
            .collect()
    )
}

async fn get_lol_match_participant_identities(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<LolParticipantIdentityDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT lmpi.*
            FROM squadov.lol_match_participant_identities AS lmpi
            WHERE lmpi.match_uuid = $1
            ",
            match_uuid,
        )
            .fetch_all(&*ex)
            .await?
            .into_iter()
            .map(|x| {
                let has_player = x.account_id.is_some() &&
                    x.current_account_id.is_some() &&
                    x.current_platform_id.is_some() &&
                    x.summoner_name.is_some() &&
                    x.platform_id.is_some();

                LolParticipantIdentityDto{
                    participant_id: x.participant_id,
                    player: if has_player {
                        Some(LolPlayerDto{
                            account_id: x.account_id.unwrap(),
                            current_account_id: x.current_account_id.unwrap(),
                            current_platform_id: x.current_platform_id.unwrap(),
                            summoner_name: x.summoner_name.unwrap(),
                            summoner_id: x.summoner_id,
                            platform_id: x.platform_id.unwrap(),
                        })
                    } else {
                        None
                    }
                }
            })
            .collect()
    )
}

async fn get_lol_match_participants(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<LolParticipantDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT lmp.*
            FROM squadov.lol_match_participants AS lmp
            WHERE lmp.match_uuid = $1
            ",
            match_uuid,
        )
            .fetch_all(&*ex)
            .await?
            .into_iter()
            .map(|x| {
                LolParticipantDto{
                    participant_id: x.participant_id,
                    champion_id: x.champion_id,
                    team_id: x.team_id,
                    spell1_id: x.spell1_id,
                    spell2_id: x.spell2_id,
                    stats: LolParticipantStatsDto{
                        participant_id: x.participant_id,
                        champ_level: x.champ_level,
                        win: x.win,
                        // KDA
                        kills: x.kills,
                        deaths: x.deaths,
                        assists: x.assists,
                        // Items
                        item0: x.item0,
                        item1: x.item1,
                        item2: x.item2,
                        item3: x.item3,
                        item4: x.item4,
                        item5: x.item5,
                        item6: x.item6,
                        // Notable kills
                        double_kills: x.double_kills,
                        triple_kills: x.triple_kills,
                        quadra_kills: x.quadra_kills,
                        penta_kills: x.penta_kills,
                        first_blood_kill: x.first_blood_kill,
                        // Econ Stats
                        gold_earned: x.gold_earned,
                        gold_spent: x.gold_spent,
                        // Neutral Stats
                        neutral_minions_killed_team_jungle: x.neutral_minions_killed_team_jungle,
                        neutral_minions_killed_enemy_jungle: x.neutral_minions_killed_enemy_jungle,
                        wards_killed: x.wards_killed,
                        wards_placed: x.wards_placed,
                        vision_wards_bought_in_game: x.vision_wards_bought_in_game,
                        sight_wards_bought_in_game: x.sight_wards_bought_in_game,
                        neutral_minions_kills: x.neutral_minions_kills,
                        total_minions_killed: x.total_minions_killed,
                        // Objective Stats
                        damage_dealt_to_objectives: x.damage_dealt_to_objectives,
                        inhibitor_kills: x.inhibitor_kills,
                        turret_kills: x.turret_kills,
                        damage_dealt_to_turrets: x.damage_dealt_to_turrets,
                        // Score
                        total_player_score: x.total_player_score,
                        total_score_rank: x.total_score_rank,
                        objective_player_score: x.objective_player_score,
                        combat_player_score: x.combat_player_score,
                        vision_score: x.vision_score,
                        // Damage Dealt to Champions
                        total_damage_dealt_to_champions: x.total_damage_dealt_to_champions,
                        physical_damage_dealt_to_champions: x.physical_damage_dealt_to_champions,
                        magic_damage_dealt_to_champions: x.magic_damage_dealt_to_champions,
                        true_damage_dealt_to_champions: x.true_damage_dealt_to_champions,
                        // Damage Dealt
                        total_damage_dealt: x.total_damage_dealt,
                        physical_damage_dealt: x.physical_damage_dealt,
                        magic_damage_dealt: x.magic_damage_dealt,
                        true_damage_dealt: x.true_damage_dealt,
                        // Damage Taken
                        total_damage_taken: x.total_damage_taken,
                        physical_damage_taken: x.physical_damage_taken,
                        magical_damage_taken: x.magical_damage_taken,
                        true_damage_taken: x.true_damage_taken,
                        // Other Combat
                        total_heal: x.total_heal,
                        damage_self_mitigated: x.damage_self_mitigated,
                    },
                    timeline: LolParticipantTimelineDto{
                        participant_id: x.participant_id,
                        lane: x.lane,
                    },
                }
            })
            .collect()
    )
}

async fn get_lol_match_info(ex: &PgPool, match_uuid: &Uuid) -> Result<LolMatchDto, SquadOvError> {
    let base_info = sqlx::query!(
        "
        SELECT lmi.*
        FROM squadov.lol_match_info AS lmi
        WHERE lmi.match_uuid = $1
        ",
        match_uuid
    )
        .fetch_one(&*ex)
        .await?;

    Ok(LolMatchDto{
        game_id: base_info.game_id,
        queue_id: base_info.queue_id,
        game_type: base_info.game_type,
        game_duration: base_info.game_duration,
        platform_id: base_info.platform_id,
        game_creation: Some(base_info.game_creation),
        season_id: base_info.season_id,
        game_version: base_info.game_version,
        map_id: base_info.map_id,
        game_mode: base_info.game_mode,
        participant_identities: get_lol_match_participant_identities(&*ex, match_uuid).await?,
        teams: get_lol_match_teams(&*ex, match_uuid).await?,
        participants: get_lol_match_participants(&*ex, match_uuid).await?,
    })
}

pub async fn get_lol_match(ex: &PgPool, match_uuid: &Uuid) -> Result<FullLolMatch, SquadOvError> {
    Ok(FullLolMatch{
        lol_match: get_lol_match_info(&*ex, match_uuid).await?,
        timeline: get_lol_match_timeline(&*ex, match_uuid).await?,
        user_id_to_participant_id: sqlx::query!(
            "
            SELECT ral.user_id, lmpi.participant_id
            FROM squadov.lol_match_participant_identities AS lmpi
            INNER JOIN squadov.riot_accounts AS ra
                ON ra.summoner_id = lmpi.summoner_id
            INNER JOIN squadov.riot_account_links AS ral
                ON ral.puuid = ra.puuid
            WHERE lmpi.match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(&*ex)
            .await?
            .into_iter()
            .map(|x| {
                (x.user_id, x.participant_id)
            })
            .collect(),
    })
}