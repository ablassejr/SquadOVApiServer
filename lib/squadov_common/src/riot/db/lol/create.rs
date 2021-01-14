use crate::{
    SquadOvError,
    matches,
    riot::games::{
        LolMatchDto,
        LolMatchTimelineDto,
        LolParticipantIdentityDto,
        LolTeamStatsDto,
        LolTeamBansDto,
        LolParticipantDto,
        LolMatchParticipantFrameDto,
        LolMatchEventDto
    }
};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;

async fn link_match_uuid_to_lol_match(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, platform: &str, game_id: i64) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.lol_matches (
            match_uuid,
            platform,
            match_id
        )
        VALUES (
            $1,
            $2,
            $3
        )
        ",
        match_uuid,
        platform,
        game_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_or_get_match_uuid_for_lol_match(ex: &mut Transaction<'_, Postgres>, platform: &str, game_id: i64) -> Result<Uuid, SquadOvError> {
    Ok(match super::get_lol_match_uuid_if_exists(&mut *ex, platform, game_id).await? {
        Some(x) => x,
        None => {
            let match_uuid = matches::create_new_match(&mut *ex).await?;
            link_match_uuid_to_lol_match(&mut *ex, &match_uuid, platform, game_id).await?;
            match_uuid
        }
    })
}

async fn store_lol_match_participant_identities(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, iden: &[LolParticipantIdentityDto]) -> Result<(), SquadOvError> {
    if iden.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_participant_identities (
                match_uuid,
                participant_id,
                account_id,
                current_account_id,
                current_platform_id,
                summoner_name,
                summoner_id,
                platform_id
            )
            VALUES
        ".to_string()
    ];

    for id in iden {
        let player_ref = id.player.as_ref();
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {participant_id},
                    {account_id},
                    {current_account_id},
                    {current_platform_id},
                    {summoner_name},
                    {summoner_id},
                    {platform_id}
                )
                ",
                match_uuid=match_uuid,
                participant_id=id.participant_id,
                account_id=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.account_id.clone()) })),
                current_account_id=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.current_account_id.clone()) })),
                current_platform_id=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.current_platform_id.clone()) })),
                summoner_name=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.summoner_name.clone()) })),
                summoner_id=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.summoner_id.clone()) })),
                platform_id=crate::sql_format_option_string(&player_ref.and_then(|x| { Some(x.platform_id.clone()) })),
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

struct WrappedTeamBan<'a> {
    team_id: i32,
    base: &'a LolTeamBansDto
}

async fn store_lol_match_teams(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, teams: &[LolTeamStatsDto]) -> Result<(), SquadOvError> {
    if teams.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_teams (
                match_uuid,
                team_id,
                tower_kills,
                rift_herald_kills,
                first_blood,
                inhibitor_kills,
                first_baron,
                first_dragon,
                dragon_kills,
                baron_kills,
                first_inhibitor,
                first_tower,
                first_rift_herald,
                win
            )
            VALUES
        ".to_string()
    ];
    
    let mut all_bans: Vec<WrappedTeamBan> = Vec::new();
    for t in teams {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {team_id},
                    {tower_kills},
                    {rift_herald_kills},
                    {first_blood},
                    {inhibitor_kills},
                    {first_baron},
                    {first_dragon},
                    {dragon_kills},
                    {baron_kills},
                    {first_inhibitor},
                    {first_tower},
                    {first_rift_herald},
                    '{win}'
                )
            ",
                match_uuid=match_uuid,
                team_id=t.team_id,
                tower_kills=t.tower_kills,
                rift_herald_kills=t.rift_herald_kills,
                first_blood=crate::sql_format_bool(t.first_blood),
                inhibitor_kills=t.inhibitor_kills,
                first_baron=crate::sql_format_bool(t.first_baron),
                first_dragon=crate::sql_format_bool(t.first_dragon),
                dragon_kills=t.dragon_kills,
                baron_kills=t.baron_kills,
                first_inhibitor=crate::sql_format_bool(t.first_inhibitor),
                first_tower=crate::sql_format_bool(t.first_tower),
                first_rift_herald=crate::sql_format_bool(t.first_rift_herald),
                win=&t.win,
            )
        );
        sql.push(",".to_string());

        all_bans.extend(t.bans.iter().map(|x| {
            WrappedTeamBan{
                team_id: t.team_id,
                base: x,
            }
        }));
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;

    store_lol_match_team_bans(&mut *ex, match_uuid, &all_bans).await?;
    Ok(())
}

async fn store_lol_match_team_bans<'a>(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, bans: &[WrappedTeamBan<'a>]) -> Result<(), SquadOvError> {
    if bans.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_bans (
                match_uuid,
                team_id,
                champion_id,
                pick_turn
            )
            VALUES
        ".to_string()
    ];

    for b in bans {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {team_id},
                    {champion_id},
                    {pick_turn}
                )
            ",
                match_uuid=match_uuid,
                team_id=b.team_id,
                champion_id=b.base.champion_id,
                pick_turn=b.base.pick_turn,
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

async fn store_lol_match_participants(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, participants: &[LolParticipantDto]) -> Result<(), SquadOvError> {
    if participants.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_participants (
                match_uuid,
                participant_id,
                champion_id,
                team_id,
                spell1_id,
                spell2_id ,
                champ_level,
                win,
                kills,
                deaths,
                assists,
                item0,
                item1,
                item2,
                item3,
                item4,
                item5,
                item6,
                double_kills,
                triple_kills,
                quadra_kills,
                penta_kills,
                first_blood_kill,
                gold_earned,
                gold_spent,
                neutral_minions_killed_team_jungle,
                neutral_minions_killed_enemy_jungle,
                wards_killed,
                wards_placed,
                vision_wards_bought_in_game,
                sight_wards_bought_in_game,
                neutral_minions_kills,
                total_minions_killed,
                damage_dealt_to_objectives,
                inhibitor_kills,
                turret_kills,
                damage_dealt_to_turrets,
                total_player_score,
                total_score_rank,
                objective_player_score,
                combat_player_score,
                vision_score,
                total_damage_dealt_to_champions,
                physical_damage_dealt_to_champions,
                magic_damage_dealt_to_champions,
                true_damage_dealt_to_champions,
                total_damage_dealt,
                physical_damage_dealt,
                magic_damage_dealt, 
                true_damage_dealt,
                total_damage_taken, 
                physical_damage_token,
                magical_damage_taken,
                true_damage_taken,
                total_heal,
                damage_self_mitigated
            )
            VALUES
        ".to_string()
    ];

    for p in participants {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {participant_id},
                    {champion_id},
                    {team_id},
                    {spell1_id},
                    {spell2_id },
                    {champ_level},
                    {win},
                    {kills},
                    {deaths},
                    {assists},
                    {item0},
                    {item1},
                    {item2},
                    {item3},
                    {item4},
                    {item5},
                    {item6},
                    {double_kills},
                    {triple_kills},
                    {quadra_kills},
                    {penta_kills},
                    {first_blood_kill},
                    {gold_earned},
                    {gold_spent},
                    {neutral_minions_killed_team_jungle},
                    {neutral_minions_killed_enemy_jungle},
                    {wards_killed},
                    {wards_placed},
                    {vision_wards_bought_in_game},
                    {sight_wards_bought_in_game},
                    {neutral_minions_kills},
                    {total_minions_killed},
                    {damage_dealt_to_objectives},
                    {inhibitor_kills},
                    {turret_kills},
                    {damage_dealt_to_turrets},
                    {total_player_score},
                    {total_score_rank},
                    {objective_player_score},
                    {combat_player_score},
                    {vision_score},
                    {total_damage_dealt_to_champions},
                    {physical_damage_dealt_to_champions},
                    {magic_damage_dealt_to_champions},
                    {true_damage_dealt_to_champions},
                    {total_damage_dealt},
                    {physical_damage_dealt},
                    {magic_damage_dealt}, 
                    {true_damage_dealt},
                    {total_damage_taken}, 
                    {physical_damage_token},
                    {magical_damage_taken},
                    {true_damage_taken},
                    {total_heal},
                    {damage_self_mitigated}
                )
            ",
                match_uuid=match_uuid,
                participant_id=p.participant_id,
                champion_id=p.champion_id,
                team_id=p.team_id,
                spell1_id=p.spell1_id,
                spell2_id =p.spell2_id ,
                champ_level=p.stats.champ_level,
                win=p.stats.win,
                kills=p.stats.kills,
                deaths=p.stats.deaths,
                assists=p.stats.assists,
                item0=p.stats.item0,
                item1=p.stats.item1,
                item2=p.stats.item2,
                item3=p.stats.item3,
                item4=p.stats.item4,
                item5=p.stats.item5,
                item6=p.stats.item6,
                double_kills=p.stats.double_kills,
                triple_kills=p.stats.triple_kills,
                quadra_kills=p.stats.quadra_kills,
                penta_kills=p.stats.penta_kills,
                first_blood_kill=p.stats.first_blood_kill,
                gold_earned=p.stats.gold_earned,
                gold_spent=p.stats.gold_spent,
                neutral_minions_killed_team_jungle=p.stats.neutral_minions_killed_team_jungle,
                neutral_minions_killed_enemy_jungle=p.stats.neutral_minions_killed_enemy_jungle,
                wards_killed=p.stats.wards_killed,
                wards_placed=p.stats.wards_placed,
                vision_wards_bought_in_game=p.stats.vision_wards_bought_in_game,
                sight_wards_bought_in_game=p.stats.sight_wards_bought_in_game,
                neutral_minions_kills=p.stats.neutral_minions_kills,
                total_minions_killed=p.stats.total_minions_killed,
                damage_dealt_to_objectives=p.stats.damage_dealt_to_objectives,
                inhibitor_kills=p.stats.inhibitor_kills,
                turret_kills=p.stats.turret_kills,
                damage_dealt_to_turrets=p.stats.damage_dealt_to_turrets,
                total_player_score=p.stats.total_player_score,
                total_score_rank=p.stats.total_score_rank,
                objective_player_score=p.stats.objective_player_score,
                combat_player_score=p.stats.combat_player_score,
                vision_score=p.stats.vision_score,
                total_damage_dealt_to_champions=p.stats.total_damage_dealt_to_champions,
                physical_damage_dealt_to_champions=p.stats.physical_damage_dealt_to_champions,
                magic_damage_dealt_to_champions=p.stats.magic_damage_dealt_to_champions,
                true_damage_dealt_to_champions=p.stats.true_damage_dealt_to_champions,
                total_damage_dealt=p.stats.total_damage_dealt,
                physical_damage_dealt=p.stats.physical_damage_dealt,
                magic_damage_dealt=p.stats.magic_damage_dealt ,
                true_damage_dealt=p.stats.true_damage_dealt,
                total_damage_taken=p.stats.total_damage_taken ,
                physical_damage_token=p.stats.physical_damage_token,
                magical_damage_taken=p.stats.magical_damage_taken,
                true_damage_taken=p.stats.true_damage_taken,
                total_heal=p.stats.total_heal,
                damage_self_mitigated=p.stats.damage_self_mitigated
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

pub async fn store_lol_match_info(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, lol_match: &LolMatchDto) -> Result<(), SquadOvError> {
    // This must ABSOLUTELY fail when it detects a conflict as we should be able safely assume that the data we'll get from the 
    // match history endpoint is the same every time. Therefore, any duplicates here would be redundant and furthermore, the events
    // we store aren't really unique so if we continue we'd get actual duplicated data there.
    sqlx::query!(
        "
        INSERT INTO squadov.lol_match_info (
            match_uuid,
            game_id,
            platform_id,
            queue_id,
            game_type,
            game_duration,
            game_creation,
            season_id,
            game_version,
            map_id,
            game_mode
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            $9,
            $10,
            $11
        )
        ",
        match_uuid,
        lol_match.game_id,
        &lol_match.platform_id,
        lol_match.queue_id,
        &lol_match.game_type,
        lol_match.game_duration,
        &lol_match.game_creation.ok_or(SquadOvError::BadRequest)?,
        lol_match.season_id,
        &lol_match.game_version,
        lol_match.map_id,
        &lol_match.game_mode
    )
        .execute(&mut *ex)
        .await?;
    
    // The ordering here is pretty essential as players has foreign keys pointing to both teams and identities.
    store_lol_match_participant_identities(&mut *ex, match_uuid, &lol_match.participant_identities).await?;
    store_lol_match_teams(&mut *ex, match_uuid, &lol_match.teams).await?;
    store_lol_match_participants(&mut *ex, match_uuid, &lol_match.participants).await?;
    Ok(())
}

struct WrappedParticipantFrame<'a> {
    timestamp: i64,
    base: &'a LolMatchParticipantFrameDto
}

struct WrappedEvent<'a> {
    base: &'a LolMatchEventDto
}

async fn store_lol_match_timeline_participant_frames<'a>(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, frames: &[WrappedParticipantFrame<'a>]) -> Result<(), SquadOvError> {
    if frames.is_empty() {
        return Ok(());
    }
    
    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_timeline_participant_frames (
                match_uuid,
                timestamp,
                participant_id,
                minions_killed,
                total_gold,
                level,
                xp,
                current_gold,
                jungle_minions_killed,
                x,
                y
            )
            VALUES
        ".to_string()
    ];

    for f in frames {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {timestamp},
                    {participant_id},
                    {minions_killed},
                    {total_gold},
                    {level},
                    {xp},
                    {current_gold},
                    {jungle_minions_killed},
                    {x},
                    {y}
                )
            ",
                match_uuid=match_uuid,
                timestamp=f.timestamp,
                participant_id=f.base.participant_id,
                minions_killed=f.base.minions_killed,
                total_gold=f.base.total_gold,
                level=f.base.level,
                xp=f.base.xp,
                current_gold=f.base.current_gold,
                jungle_minions_killed=f.base.jungle_minions_killed,
                x=crate::sql_format_option_value(&f.base.position.as_ref().and_then(|x| { Some(x.x) })),
                y=crate::sql_format_option_value(&f.base.position.as_ref().and_then(|x| { Some(x.x) })),
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

async fn store_lol_match_timeline_events<'a>(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, events: &[WrappedEvent<'a>]) -> Result<(), SquadOvError> {
    if events.is_empty() {
        return Ok(());
    }

    let mut sql = vec![
        "
            INSERT INTO squadov.lol_match_timeline_events (
                match_uuid,
                timestamp,
                real_type,
                lane_type,
                skill_slot,
                ascended_type,
                creator_id,
                after_id,
                event_type,
                level_up_type,
                ward_type,
                participant_id,
                tower_type,
                item_id,
                before_id,
                monster_type,
                monster_sub_type,
                team_id,
                x,
                y,
                killer_id,
                assisting_participant_ids,
                building_type,
                victim_id
            )
            VALUES
        ".to_string()
    ];

    for e in events {
        sql.push(
            format!("
                (
                    '{match_uuid}',
                    {timestamp},
                    '{real_type}',
                    {lane_type},
                    {skill_slot},
                    {ascended_type},
                    {creator_id},
                    {after_id},
                    {event_type},
                    {level_up_type},
                    {ward_type},
                    {participant_id},
                    {tower_type},
                    {item_id},
                    {before_id},
                    {monster_type},
                    {monster_sub_type},
                    {team_id},
                    {x},
                    {y},
                    {killer_id},
                    {assisting_participant_ids},
                    {building_type},
                    {victim_id}
                )
            ",
                match_uuid=match_uuid,
                timestamp=e.base.timestamp,
                real_type=&e.base.real_type,
                lane_type=crate::sql_format_option_string(&e.base.lane_type),
                skill_slot=crate::sql_format_option_value(&e.base.skill_slot),
                ascended_type=crate::sql_format_option_string(&e.base.ascended_type),
                creator_id=crate::sql_format_option_value(&e.base.creator_id),
                after_id=crate::sql_format_option_value(&e.base.after_id),
                event_type=crate::sql_format_option_string(&e.base.event_type),
                level_up_type=crate::sql_format_option_string(&e.base.level_up_type),
                ward_type=crate::sql_format_option_string(&e.base.ward_type),
                participant_id=crate::sql_format_option_value(&e.base.participant_id),
                tower_type=crate::sql_format_option_string(&e.base.tower_type),
                item_id=crate::sql_format_option_value(&e.base.item_id),
                before_id=crate::sql_format_option_value(&e.base.before_id),
                monster_type=crate::sql_format_option_string(&e.base.monster_type),
                monster_sub_type=crate::sql_format_option_string(&e.base.monster_sub_type),
                team_id=crate::sql_format_option_value(&e.base.team_id),
                x=crate::sql_format_option_value(&e.base.position.as_ref().and_then(|x| { Some(x.x) })),
                y=crate::sql_format_option_value(&e.base.position.as_ref().and_then(|x| { Some(x.y) })),
                killer_id=crate::sql_format_option_value(&e.base.killer_id),
                assisting_participant_ids=crate::sql_format_integer_array(&e.base.assisting_participant_ids.as_ref().unwrap_or(&vec![])),
                building_type=crate::sql_format_option_string(&e.base.building_type),
                victim_id=crate::sql_format_option_string(&e.base.victim_id),
            )
        );
        sql.push(",".to_string());
    }

    sql.truncate(sql.len() - 1);
    sqlx::query(&sql.join("")).execute(&mut *ex).await?;
    Ok(())
}

pub async fn store_lol_match_timeline_info(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, timeline: &LolMatchTimelineDto) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.lol_match_timeline (
            match_uuid,
            frame_interval
        )
        VALUES (
            $1,
            $2
        )
        ",
        match_uuid,
        timeline.frame_interval,
    )
        .execute(&mut *ex)
        .await?;

    let mut frames: Vec<WrappedParticipantFrame> = Vec::new();
    let mut events: Vec<WrappedEvent> = Vec::new();

    for f in &timeline.frames {
        frames.extend(f.participant_frames.iter().map(|(_, x)| {
            WrappedParticipantFrame {
                timestamp: f.timestamp,
                base: x,
            }
        }));

        events.extend(f.events.iter().map(|x| {
            WrappedEvent {
                base: x,
            }
        }));
    }

    store_lol_match_timeline_participant_frames(&mut *ex, match_uuid, &frames).await?;
    store_lol_match_timeline_events(&mut *ex, match_uuid, &events).await?;
    Ok(())
}