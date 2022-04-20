use crate::{
    SquadOvError,
    riot::games::{
        FullLolMatch,
        LolMatchDto,
        LolMatchInfoDto,
        LolMatchMetadataDto,
        LolMatchTimelineDto,
        LolParticipantDto,
        LolMatchFrameDto,
        LolMatchParticipantFrameDto,
        LolMatchEventDto,
        LolMatchPositionDto,
        LolTeamDto,
        LolBanDto,
        LolObjectivesDto,
        LolSingleObjectiveDto,
        LolMatchTimelineInfoDto,
    },
};
use sqlx::{Executor, Postgres, PgPool};
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
                participant_frames: frame_map.remove(&timestamp).unwrap_or(vec![]).into_iter().map(|x| {
                    (x.participant_id.to_string(), x)
                }).collect(),
                events: event_map.remove(&timestamp).unwrap_or(vec![]),
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
            metadata: LolMatchMetadataDto::default(),
            info: LolMatchTimelineInfoDto{
                frame_interval: data.frame_interval,
                frames: get_lol_match_timeline_frames(&*ex, match_uuid).await?,
            },
        }
    )
}

pub async fn get_lol_match_region<'a, T>(ex: T, match_uuid: &Uuid) -> Result<String, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT platform_id
            FROM squadov.lol_match_info
            WHERE match_uuid = $1
            "#,
            match_uuid,
        )
            .fetch_one(ex)
            .await?
            .platform_id
    )
}

pub async fn get_lol_match_teams<'a, T>(ex: T, match_uuid: &Uuid) -> Result<Vec<LolTeamDto>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let mut bans_per_team: HashMap<i32, Vec<LolBanDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT lmb.*
        FROM squadov.lol_match_bans AS lmb
        WHERE lmb.match_uuid = $1
        ",
        match_uuid
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !bans_per_team.contains_key(&x.team_id) {
                bans_per_team.insert(x.team_id, Vec::new());
            }
            let vec = bans_per_team.get_mut(&x.team_id).unwrap();
            vec.push(LolBanDto{
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
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                LolTeamDto{
                    objectives: LolObjectivesDto {
                        baron: LolSingleObjectiveDto{
                            first: x.first_baron,
                            kills: x.baron_kills,
                        },
                        champion: LolSingleObjectiveDto{
                            first: x.first_blood,
                            kills: 0,
                        },
                        dragon: LolSingleObjectiveDto{
                            first: x.first_dragon,
                            kills: x.dragon_kills,
                        },
                        inhibitor: LolSingleObjectiveDto{
                            first: x.first_inhibitor,
                            kills: x.inhibitor_kills,
                        },
                        rift_herald: LolSingleObjectiveDto{
                            first: x.first_rift_herald,
                            kills: x.rift_herald_kills,
                        },
                        tower: LolSingleObjectiveDto{
                            first: x.first_tower,
                            kills: x.tower_kills,
                        },
                    },
                    team_id: x.team_id,
                    win: x.win == "Win",
                    bans: bans_per_team.remove(&x.team_id).unwrap_or(vec![]),
                }
            })
            .collect()
    )
}

pub async fn get_lol_match_participants<'a, T>(ex: T, match_uuid: &Uuid) -> Result<Vec<LolParticipantDto>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT lmp.*, lmpi.puuid, lmpi.summoner_name, lmpi.summoner_id
            FROM squadov.lol_match_participants AS lmp
            INNER JOIN squadov.lol_match_participant_identities AS lmpi
                ON lmpi.match_uuid = lmp.match_uuid
                    AND lmpi.participant_id = lmp.participant_id
            WHERE lmp.match_uuid = $1
            ",
            match_uuid,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                LolParticipantDto{
                    assists: x.assists as i32,
                    champ_level: x.champ_level as i32,
                    champion_id: x.champion_id as i32,
                    damage_dealt_to_objectives: x.damage_dealt_to_objectives as i32,
                    damage_dealt_to_turrets: x.damage_dealt_to_turrets as i32,
                    damage_self_mitigated: x.damage_self_mitigated as i32,
                    deaths: x.deaths as i32,
                    double_kills: x.double_kills as i32,
                    first_blood_kill: x.first_blood_kill,
                    gold_earned: x.gold_earned as i32,
                    gold_spent: x.gold_spent as i32,
                    inhibitor_kills: x.inhibitor_kills as i32,
                    item0: x.item0 as i32,
                    item1: x.item1 as i32,
                    item2: x.item2 as i32,
                    item3: x.item3 as i32,
                    item4: x.item4 as i32,
                    item5: x.item5 as i32,
                    item6: x.item6 as i32,
                    kills: x.kills as i32,
                    lane: x.lane,
                    magic_damage_dealt: x.magic_damage_dealt as i32,
                    magic_damage_dealt_to_champions: x.magic_damage_dealt_to_champions as i32,
                    magic_damage_taken: x.magical_damage_taken as i32,
                    neutral_minions_killed: x.neutral_minions_kills as i32,
                    participant_id: x.participant_id as i32,
                    penta_kills: x.penta_kills as i32,
                    physical_damage_dealt: x.physical_damage_dealt as i32,
                    physical_damage_dealt_to_champions: x.physical_damage_dealt_to_champions as i32,
                    physical_damage_taken: x.physical_damage_taken as i32,
                    puuid: x.puuid.unwrap_or(String::new()),
                    quadra_kills: x.quadra_kills as i32,
                    sight_wards_bought_in_game: x.sight_wards_bought_in_game as i32,
                    summoner1_id: x.spell1_id as i32,
                    summoner2_id: x.spell2_id as i32,
                    summoner_id: x.summoner_id.unwrap_or(String::new()),
                    summoner_name: x.summoner_name.unwrap_or(String::new()),
                    team_id: x.team_id as i32,
                    total_damage_dealt: x.total_damage_dealt as i32,
                    total_damage_dealt_to_champions: x.total_damage_dealt_to_champions as i32,
                    total_damage_taken: x.total_damage_dealt_to_champions as i32,
                    total_heal: x.total_heal as i32,
                    total_minions_killed: x.total_minions_killed as i32,
                    triple_kills: x.triple_kills as i32,
                    true_damage_dealt: x.true_damage_dealt as i32,
                    true_damage_dealt_to_champions: x.true_damage_dealt_to_champions as i32,
                    true_damage_taken: x.true_damage_taken as i32,
                    turret_kills: x.turret_kills as i32,
                    vision_score: x.vision_score as i32,
                    vision_wards_bought_in_game: x.vision_wards_bought_in_game as i32,
                    wards_killed: x.wards_killed as i32,
                    wards_placed: x.wards_placed as i32,
                    win: x.win,
                    ..LolParticipantDto::default()
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
        metadata: LolMatchMetadataDto{
            match_id: format!("{}_{}", &base_info.platform_id, base_info.game_id),
            ..LolMatchMetadataDto::default()
        },
        info: LolMatchInfoDto{
            game_creation: Some(base_info.game_creation),
            game_duration: base_info.game_duration,
            game_id: base_info.game_id,
            game_mode: base_info.game_mode,
            game_type: base_info.game_type,
            game_version: base_info.game_version,
            map_id: base_info.map_id,
            queue_id: base_info.queue_id,
            participants: get_lol_match_participants(ex, match_uuid).await?,
            teams: get_lol_match_teams(ex, match_uuid).await?,
            ..LolMatchInfoDto::default()
        },
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
        game_start_time: sqlx::query!(
            "
            SELECT lm.game_start_time
            FROM squadov.lol_matches AS lm
            WHERE lm.match_uuid = $1
            ",
            match_uuid
        )
            .fetch_one(&*ex)
            .await?.game_start_time
    })
}