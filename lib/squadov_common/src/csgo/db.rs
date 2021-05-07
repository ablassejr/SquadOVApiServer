use crate::SquadOvError;
use crate::csgo::{
    demo::CsgoDemo,
    gsi::CsgoGsiMatchState,
    schema::{CsgoView, CsgoCommonEventContainer, CsgoCommonPlayer, CsgoCommonRound},
    summary::CsgoPlayerMatchSummary,
};
use crate::matches::MatchPlayerPair;
use sqlx::{Transaction, Executor, Postgres};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::collections::HashSet;

pub async fn list_csgo_match_summaries_for_uuids<'a, T>(ex: T, uuids: &[MatchPlayerPair]) -> Result<Vec<CsgoPlayerMatchSummary>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let match_uuids = uuids.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
    let player_uuids = uuids.iter().map(|x| { x.player_uuid.clone() }).collect::<Vec<Uuid>>();

    Ok(
        sqlx::query_as!(
            CsgoPlayerMatchSummary,
            r#"
            SELECT
                inp.match_uuid AS "match_uuid!",
                inp.user_uuid AS "user_uuid!",
                cmv.map AS "map!",
                cmv.mode AS "mode!",
                cmv.start_time AS "match_start_time!",
                COALESCE(EXTRACT(EPOCH FROM cmv.stop_time - cmv.start_time), 0)::INTEGER AS "match_length_seconds!",
                cecp.kills AS "kills!",
                cecp.deaths AS "deaths!",
                cecp.assists AS "assists!",
                cecp.mvps AS "mvps!",
                COALESCE(winner.win, FALSE) AS "winner!",
                cec.event_source = 1 AS "has_demo!",
                (COUNT(crd.*) FILTER(WHERE crd.hitgroup = 1))::INTEGER AS "headshots!",
                (COUNT(crd.*) FILTER(WHERE crd.hitgroup >= 2 OR crd.hitgroup <= 5))::INTEGER AS "bodyshots!",
                (COUNT(crd.*) FILTER(WHERE crd.hitgroup > 5))::INTEGER AS "legshots!",
                COALESCE(((SUM(crd.damage_health) + SUM(crd.damage_armor))::DOUBLE PRECISION / (winner.last_round+1)::DOUBLE PRECISION), 0.0) AS "damage_per_round!"
            FROM UNNEST($1::UUID[], $2::UUID[]) AS inp(match_uuid, user_uuid)
            INNER JOIN squadov.users AS u
                ON u.uuid = inp.user_uuid
            INNER JOIN squadov.csgo_match_views AS cmv
                ON cmv.match_uuid = inp.match_uuid
                    AND cmv.user_id = u.id
            CROSS JOIN LATERAL (
                SELECT cec.id, cec.event_source
                FROM squadov.csgo_event_container AS cec
                WHERE cec.view_uuid = cmv.view_uuid
                ORDER BY cec.event_source DESC
                LIMIT 1
            ) AS cec
            CROSS JOIN LATERAL (
                SELECT sul.steam_id
                FROM squadov.steam_user_links AS sul
                WHERE sul.user_id = u.id
                LIMIT 1
            ) AS sul
            INNER JOIN squadov.csgo_event_container_players AS cecp
                ON cecp.container_id = cec.id AND cecp.steam_id = sul.steam_id
            LEFT JOIN LATERAL (
                SELECT ccr.winning_team = cps.team, ccr.round_num
                FROM squadov.csgo_event_container_rounds AS ccr
                INNER JOIN squadov.csgo_event_container_round_player_stats AS cps
                    ON cps.container_id = ccr.container_id
                        AND cps.round_num = ccr.round_num
                        AND user_id = cecp.user_id
                WHERE ccr.container_id = cec.id
                ORDER BY ccr.round_num DESC
                LIMIT 1
            ) AS winner(win, last_round) ON TRUE
            LEFT JOIN squadov.csgo_event_container_round_damage AS crd
                ON crd.container_id = cec.id
                    AND crd.attacker = cecp.user_id
            GROUP BY
                inp.match_uuid,
                inp.user_uuid,
                cmv.map,
                cmv.mode,
                cmv.start_time,
                "match_length_seconds!",
                cecp.kills,
                cecp.deaths,
                cecp.assists,
                cecp.mvps,
                winner.win,
                winner.last_round,
                cec.event_source
            "#,
            &match_uuids,
            &player_uuids,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn create_csgo_view_for_user(ex: &mut Transaction<'_, Postgres>, user_id: i64, server: &str, start_time: &DateTime<Utc>, map: &str, mode: &str) -> Result<Uuid, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            INSERT INTO squadov.csgo_match_views (
                view_uuid,
                user_id,
                game_server,
                start_time,
                map,
                mode
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4,
                $5
            )
            RETURNING view_uuid
            ",
            user_id,
            server,
            start_time,
            map,
            mode,
        )
            .fetch_one(ex)
            .await?
            .view_uuid
    )
}

pub async fn find_csgo_view<'a, T>(ex: T, view_uuid: &Uuid) -> Result<CsgoView, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CsgoView,
            "
            SELECT *
            FROM squadov.csgo_match_views
            WHERE view_uuid = $1
            ",
            view_uuid,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn find_existing_csgo_match<'a, T>(ex: T, server: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT match_uuid
            FROM squadov.csgo_matches
            WHERE connected_server = $1
                AND tr && tstzrange($2, $3, '[]') 
            ",
            server,
            start_time,
            end_time
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.match_uuid
            })
    )
}

pub async fn create_csgo_match(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, server: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.csgo_matches (
            match_uuid,
            connected_server,
            tr
        ) VALUES (
            $1,
            $2,
            tstzrange($3, $4, '[]') 
        )
        ",
        match_uuid,
        server,
        start_time,
        end_time,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn finish_csgo_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, stop_time: &DateTime<Utc>, match_state: &CsgoGsiMatchState) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET match_uuid = $2,
            stop_time = $3
        WHERE view_uuid = $1
        ",
        view_uuid,
        match_uuid,
        stop_time
    )
        .execute(&mut *ex)
        .await?;

    store_csgo_gsi_events_for_view(ex, view_uuid, match_state).await?;
    Ok(())
}

pub async fn store_csgo_gsi_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_state: &CsgoGsiMatchState) -> Result<(), SquadOvError> {
    let common = CsgoCommonEventContainer::from_gsi(match_state)?;
    store_csgo_common_events_for_view(ex, view_uuid, &common).await?;
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET has_gsi = TRUE
        WHERE view_uuid = $1
        ",
        view_uuid
    )
        .execute(&mut *ex)
        .await?;
    Ok(())
}

pub async fn store_csgo_demo_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, demo: &CsgoDemo, ref_timestamp: &DateTime<Utc>) -> Result<(), SquadOvError> {
    let common = CsgoCommonEventContainer::from_demo(demo, ref_timestamp)?;
    store_csgo_common_events_for_view(ex, view_uuid, &common).await?;
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET has_demo = TRUE
        WHERE view_uuid = $1
        ",
        view_uuid
    )
        .execute(&mut *ex)
        .await?;
    Ok(())
}

async fn store_csgo_common_players_for_container(ex: &mut Transaction<'_, Postgres>, container_id: i64, players: &[CsgoCommonPlayer]) -> Result<HashSet<i32>, SquadOvError> {
    if players.is_empty() {
        return Ok(HashSet::new());
    }

    let mut valid_players: HashSet<i32> = HashSet::new();

    // Need to run two SQL statements here. One to insert the
    // players into the Steam account cache. And another to store the player
    // in association with this event container.
    let mut steam_sql: Vec<String> = Vec::new();
    steam_sql.push(String::from("
        INSERT INTO squadov.steam_users_cache (
            steam_id,
            steam_name
        )
        VALUES 
    "));

    let mut container_sql : Vec<String> = Vec::new();
    container_sql.push(String::from("
        INSERT INTO squadov.csgo_event_container_players (
            container_id,
            user_id,
            steam_id,
            kills,
            deaths,
            assists,
            mvps
        )
        VALUES 
    "));

    let mut added: i32 = 0;
    for p in players {
        if p.user_id != 0 && p.steam_account.steam_id != 0 {
            valid_players.insert(p.user_id);
            container_sql.push(format!("
            (
                {container_id},
                {user_id},
                {steam_id},
                {kills},
                {deaths},
                {assists},
                {mvps}
            )",
                container_id=container_id,
                user_id=p.user_id,
                steam_id=p.steam_account.steam_id,
                kills=p.kills,
                deaths=p.deaths,
                assists=p.assists,
                mvps=p.mvps,
            ));
            container_sql.push(String::from(","));

            steam_sql.push(format!("(
                {steam_id},
                {steam_name}
            )",
                steam_id=p.steam_account.steam_id,
                steam_name=crate::sql::sql_format_string(&p.steam_account.name),
            ));
            steam_sql.push(String::from(","));

            added += 1;
        }
    }

    if added > 0 {
        container_sql.truncate(container_sql.len() - 1);
        container_sql.push(String::from(" ON CONFLICT DO NOTHING"));

        steam_sql.truncate(steam_sql.len() - 1);
        steam_sql.push(String::from(" ON CONFLICT (steam_id) DO UPDATE SET steam_name=EXCLUDED.steam_name"));

        sqlx::query(&steam_sql.join("")).execute(&mut *ex).await?;
        sqlx::query(&container_sql.join("")).execute(&mut *ex).await?;
    }
    Ok(valid_players)
}

async fn store_csgo_common_rounds_for_container(ex: &mut Transaction<'_, Postgres>, container_id: i64, rounds: &[CsgoCommonRound], valid_players: &HashSet<i32>) -> Result<(), SquadOvError> {
    if rounds.is_empty() {
        return Ok(());
    }

    let mut rounds_sql: Vec<String> = Vec::new();
    rounds_sql.push(String::from("
        INSERT INTO squadov.csgo_event_container_rounds (
            container_id,
            round_num,
            tm_round_start,
            tm_round_play,
            tm_round_end,
            bomb_state,
            tm_bomb_plant,
            bomb_plant_user,
            bomb_plant_site,
            tm_bomb_event,
            bomb_event_user,
            winning_team,
            round_win_reason,
            round_mvp
        )
        VALUES 
    "));

    let mut round_stats_sql: Vec<String> = Vec::new();
    let mut added_round_stats: i32 = 0;
    round_stats_sql.push(String::from("
        INSERT INTO squadov.csgo_event_container_round_player_stats (
            container_id,
            round_num,
            user_id,
            kills,
            deaths,
            assists,
            mvp,
            equipment_value,
            headshot_kills,
            utility_damage,
            enemies_flashed,
            damage,
            armor,
            has_defuse,
            has_helmet,
            team,
            weapons
        )
        VALUES 
    "));

    let mut round_kills_sql: Vec<String> = Vec::new();
    let mut added_round_kills: i32 = 0;
    round_kills_sql.push(String::from("
        INSERT INTO squadov.csgo_event_container_round_kills (
            container_id,
            round_num,
            tm,
            victim,
            killer,
            assister,
            flash_assist,
            headshot,
            smoke,
            blind,
            wallbang,
            noscope,
            weapon
        )
        VALUES 
    "));

    let mut round_damage_sql: Vec<String> = Vec::new();
    let mut added_round_damage: i32 = 0;
    round_damage_sql.push(String::from("
        INSERT INTO squadov.csgo_event_container_round_damage (
            container_id,
            round_num,
            receiver,
            attacker,
            remaining_health,
            remaining_armor,
            damage_health,
            damage_armor,
            weapon,
            hitgroup
        )
        VALUES 
    "));

    for r in rounds {
        rounds_sql.push(format!("(
            {container_id},
            {round_num},
            {tm_round_start},
            {tm_round_play},
            {tm_round_end},
            {bomb_state},
            {tm_bomb_plant},
            {bomb_plant_user},
            {bomb_plant_site},
            {tm_bomb_event},
            {bomb_event_user},
            {winning_team},
            {round_win_reason},
            {round_mvp}
        )",
            container_id=container_id,
            round_num=r.round_num,
            tm_round_start=crate::sql::sql_format_option_some_time(r.tm_round_start.as_ref()),
            tm_round_play=crate::sql::sql_format_option_some_time(r.tm_round_play.as_ref()),
            tm_round_end=crate::sql::sql_format_option_some_time(r.tm_round_end.as_ref()),
            bomb_state=crate::sql_format_option_value(&r.bomb_state.map(|x| { x as i32 })),
            tm_bomb_plant=crate::sql::sql_format_option_some_time(r.tm_bomb_plant.as_ref()),
            bomb_plant_user=crate::sql_format_option_value(&r.bomb_plant_user),
            bomb_plant_site=crate::sql_format_option_value(&r.bomb_plant_site.map(|x| { x as i32 })),
            tm_bomb_event=crate::sql::sql_format_option_some_time(r.tm_bomb_event.as_ref()),
            bomb_event_user=crate::sql_format_option_value(&r.bomb_event_user),
            winning_team=crate::sql_format_option_value(&r.winning_team.map(|x| { x as i32 })),
            round_win_reason=crate::sql_format_option_value(&r.round_win_reason.map(|x| { x as i32 })),
            round_mvp=crate::sql_format_option_value(&r.round_mvp),
        ));
        rounds_sql.push(String::from(","));

        for ps in &r.player_stats {
            if valid_players.contains(&ps.user_id) {
                round_stats_sql.push(format!("(
                    {container_id},
                    {round_num},
                    {user_id},
                    {kills},
                    {deaths},
                    {assists},
                    {mvp},
                    {equipment_value},
                    {headshot_kills},
                    {utility_damage},
                    {enemies_flashed},
                    {damage},
                    {armor},
                    {has_defuse},
                    {has_helmet},
                    {team},
                    {weapons}
                )",
                    container_id=container_id,
                    round_num=ps.round_num,
                    user_id=ps.user_id,
                    kills=ps.kills,
                    deaths=ps.deaths,
                    assists=ps.assists,
                    mvp=crate::sql_format_bool(ps.mvp),
                    equipment_value=crate::sql_format_option_value(&ps.equipment_value),
                    headshot_kills=crate::sql_format_option_value(&ps.headshot_kills),
                    utility_damage=crate::sql_format_option_value(&ps.utility_damage),
                    enemies_flashed=crate::sql_format_option_value(&ps.enemies_flashed),
                    damage=crate::sql_format_option_value(&ps.damage),
                    armor=crate::sql_format_option_value(&ps.armor),
                    has_defuse=crate::sql_format_option_bool(ps.has_defuse),
                    has_helmet=crate::sql_format_option_bool(ps.has_helmet),
                    team=ps.team as i32,
                    weapons=crate::sql_format_integer_array(&ps.weapons.iter().map(|x| { *x as i32 }).collect::<Vec<i32>>()),
                ));
                round_stats_sql.push(String::from(","));
                added_round_stats += 1;
            }
        }
        
        for k in &r.kills {
            if let Some(killer) = k.killer {
                if !valid_players.contains(&killer) {
                    continue;
                }
            }

            if let Some(victim) = k.victim {
                if !valid_players.contains(&victim) {
                    continue;
                }
            }

            round_kills_sql.push(format!("(
                {container_id},
                {round_num},
                {tm},
                {victim},
                {killer},
                {assister},
                {flash_assist},
                {headshot},
                {smoke},
                {blind},
                {wallbang},
                {noscope},
                {weapon}
            )",
                container_id=container_id,
                round_num=k.round_num,
                tm=crate::sql_format_time(&k.tm),
                victim=crate::sql_format_option_value(&k.victim),
                killer=crate::sql_format_option_value(&k.killer),
                assister=crate::sql_format_option_value(&if let Some(assister) = k.assister {
                    if assister == 0 {
                        None
                    } else {
                        Some(assister)
                    }
                } else {
                    None
                }),
                flash_assist=crate::sql_format_option_bool(k.flash_assist),
                headshot=crate::sql_format_option_bool(k.headshot),
                smoke=crate::sql_format_option_bool(k.smoke),
                blind=crate::sql_format_option_bool(k.blind),
                wallbang=crate::sql_format_option_bool(k.wallbang),
                noscope=crate::sql_format_option_bool(k.noscope),
                weapon=crate::sql_format_option_value(&k.weapon.map(|x| { x as i32 }))
            ));
            round_kills_sql.push(String::from(","));
            added_round_kills += 1;
        }

        for d in &r.damage {
            if !valid_players.contains(&d.receiver) {
                continue;
            }

            if let Some(attacker) = d.attacker {
                if !valid_players.contains(&attacker) {
                    continue;
                }
            }

            round_damage_sql.push(format!("(
                {container_id},
                {round_num},
                {receiver},
                {attacker},
                {remaining_health},
                {remaining_armor},
                {damage_health},
                {damage_armor},
                {weapon},
                {hitgroup}
            )",
                container_id=container_id,
                round_num=d.round_num,
                receiver=d.receiver,
                attacker=crate::sql_format_option_value(&d.attacker),
                remaining_health=d.remaining_health,
                remaining_armor=d.remaining_armor,
                damage_health=d.damage_health,
                damage_armor=d.damage_armor,
                weapon=d.weapon as i32,
                hitgroup=d.hitgroup as i32,
            ));
            round_damage_sql.push(String::from(","));
            added_round_damage += 1;
        }
    }

    rounds_sql.truncate(rounds_sql.len() - 1);
    rounds_sql.push(String::from(" ON CONFLICT DO NOTHING"));
    sqlx::query(&rounds_sql.join("")).execute(&mut *ex).await?;

    if added_round_stats > 0 {
        round_stats_sql.truncate(round_stats_sql.len() - 1);
        round_stats_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&round_stats_sql.join("")).execute(&mut *ex).await?;
    }

    if added_round_kills > 0 {
        round_kills_sql.truncate(round_kills_sql.len() - 1);
        round_kills_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&round_kills_sql.join("")).execute(&mut *ex).await?;
    }

    if added_round_damage > 0 {
        round_damage_sql.truncate(round_damage_sql.len() - 1);
        round_damage_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&round_damage_sql.join("")).execute(&mut *ex).await?;
    }
    Ok(())
}

pub async fn store_csgo_common_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, events: &CsgoCommonEventContainer) -> Result<(), SquadOvError> {
    let event_container_id = sqlx::query!(
        "
        INSERT INTO squadov.csgo_event_container (
            view_uuid,
            event_source
        )
        VALUES (
            $1,
            $2
        )
        RETURNING id
        ",
        view_uuid,
        events.event_source as i32,
    )
        .fetch_one(&mut *ex)
        .await?
        .id;

    let valid_players = store_csgo_common_players_for_container(&mut *ex, event_container_id, &events.players).await?;
    store_csgo_common_rounds_for_container(&mut *ex, event_container_id, &events.rounds, &valid_players).await?;
    Ok(())
}