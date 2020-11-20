use squadov_common;
use squadov_common::hearthstone;
use squadov_common::hearthstone::{power_parser::{HearthstonePowerLogParser, HearthstoneGameState} };
use squadov_common::hearthstone::game_state::{HearthstoneGameLog, HearthstoneGameAction, HearthstoneGameSnapshot, HearthstoneGameBlock};
use squadov_common::hearthstone::GameType;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize};
use sqlx::{Transaction, Executor, Postgres};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::api::auth::SquadOVSession;
use crate::api::v1;

#[derive(Deserialize)]
pub struct CreateHearthstoneMatchInput {
    info: hearthstone::HearthstoneGameConnectionInfo,
    deck: Option<hearthstone::HearthstoneDeck>,
    players: HashMap<i32, hearthstone::HearthstonePlayer>,
    // We need a timestamp here because we need when the *user* thinks the game
    // started instead of when the message was sent to us just in case that crosses
    // a date boundary.
    timestamp: DateTime<Utc>
}

impl api::ApiApplication {
    pub async fn check_if_hearthstone_match_exists(&self, timestamp: &DateTime<Utc>, info: &hearthstone::HearthstoneGameConnectionInfo) -> Result<Option<v1::Match>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            v1::Match,
            "
            SELECT hm.match_uuid as \"uuid\"
            FROM squadov.hearthstone_matches AS hm
            WHERE hm.match_day = $1
                AND hm.server_ip = $2
                AND hm.port = $3
                AND hm.game_id = $4
            ",
            timestamp.date().naive_utc(),
            info.ip,
            info.port,
            info.game_id
        )
            .fetch_optional(&*self.pool)
            .await?)
    }

    // Creates a Hearthstone match if there's no conflict and returns the match UUID.
    pub async fn create_hearthstone_match(&self, tx : &mut Transaction<'_, Postgres>, timestamp: &DateTime<Utc>, info: &hearthstone::HearthstoneGameConnectionInfo) -> Result<Uuid, squadov_common::SquadOvError> {
        // Check if a match already exists as we want to be as close to storing 1 entry per actual server-side match as possible.
        let current_match = match self.check_if_hearthstone_match_exists(timestamp, info).await? {
            Some(x) => x,
            None => {
                // A reconnecting game should already exist in our database.
                if info.reconnecting {
                    return Err(squadov_common::SquadOvError::NotFound);
                } else {
                    let mt = self.create_new_match(tx).await?;
                    sqlx::query!(
                        "
                        INSERT INTO squadov.hearthstone_matches (
                            match_uuid,
                            server_ip,
                            port,
                            game_id,
                            match_day,
                            match_time
                        )
                        VALUES (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6
                        )
                        ",
                        mt.uuid,
                        info.ip,
                        info.port,
                        info.game_id,
                        timestamp.date().naive_utc(),
                        timestamp
                    )
                        .execute(tx)
                        .await?;
                    mt
                }
            }
        };

        Ok(current_match.uuid)
    }

    pub async fn store_hearthstone_player_medal_info(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, match_uuid: &Uuid, info: &hearthstone::HearthstoneMedalInfo, is_standard: bool) -> Result<(), squadov_common::SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_player_medals (
                match_uuid,
                player_match_id,
                league_id,
                earned_stars,
                star_level,
                best_star_level,
                win_streak,
                legend_index,
                is_standard
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
                $9
            )
            ON CONFLICT DO NOTHING
            ",
            match_uuid,
            player_match_id,
            info.league_id,
            info.earned_stars,
            info.star_level,
            info.best_star_level,
            info.win_streak,
            info.legend_index,
            is_standard
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_player(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, player: &hearthstone::HearthstonePlayer, match_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.hearthstone_match_players (
                    user_id,
                    match_uuid,
                    player_match_id,
                    player_name,
                    card_back_id,
                    arena_wins,
                    arena_loss,
                    tavern_brawl_wins,
                    tavern_brawl_loss
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
                    $9
                )
                ON CONFLICT (match_uuid, player_match_id) DO UPDATE SET
                    user_id = COALESCE(squadov.hearthstone_match_players.user_id, EXCLUDED.user_id),
                    arena_wins = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.arena_wins
                                      ELSE squadov.hearthstone_match_players.arena_wins END,
                    arena_loss = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.arena_loss
                                 ELSE squadov.hearthstone_match_players.arena_loss END,
                    tavern_brawl_wins = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.tavern_brawl_wins
                                        ELSE squadov.hearthstone_match_players.tavern_brawl_wins END,
                    tavern_brawl_loss = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.tavern_brawl_loss
                                        ELSE squadov.hearthstone_match_players.tavern_brawl_loss END
                ",
                // *ONLY* The local player should be associated with the user we pulled from the session.
                // Ideally we'd get them to OAuth with Blizzard to verify account ownership instead?
                if player.local {
                    Some(user_id)
                } else {
                    None
                },
                match_uuid,
                player_match_id,
                player.name,
                player.card_back_id,
                player.arena_wins as i32,
                player.arena_loss as i32,
                player.tavern_brawl_wins as i32,
                player.tavern_brawl_loss as i32
            )
        ).await?;

        self.store_hearthstone_player_medal_info(tx, player_match_id, match_uuid, &player.medal_info.standard, true).await?;
        self.store_hearthstone_player_medal_info(tx, player_match_id, match_uuid, &player.medal_info.wild, false).await?;
        Ok(())
    }

    pub async fn store_hearthstone_raw_power_logs(&self, tx : &mut Transaction<'_, Postgres>, logs: &[hearthstone::HearthstoneRawLog], match_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        let raw = serde_json::to_value(logs)?;
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_raw_power_logs (
                user_id,
                match_uuid,
                raw_logs,
                parsed
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            user_id,
            match_uuid,
            raw,
            false
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_metadata(&self, tx : &mut Transaction<'_, Postgres>, st: &HearthstoneGameState, uuid: &Uuid) -> Result<(), squadov_common::SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_metadata (
                match_uuid,
                game_type,
                format_type,
                scenario_id
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            uuid,
            st.game_type as i32,
            st.format_type as i32,
            st.scenario_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_actions(&self, tx: &mut Transaction<'_, Postgres>, actions: &[HearthstoneGameAction], uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        if actions.is_empty() {
            return Ok(());
        }

        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.hearthstone_actions (
                match_uuid,
                user_id,
                action_id,
                tm,
                entity_id,
                tags,
                attributes,
                parent_block,
                action_type
            )
            VALUES
        "));
        for (idx, m) in actions.iter().enumerate() {
            sql.push(format!("(
                '{match_uuid}',
                {user_id},
                {action_id},
                {tm},
                {entity_id},
                '{tags}',
                '{attributes}',
                {parent_block},
                {action_type}
            )",
                match_uuid=uuid,
                user_id=user_id,
                action_id=idx,
                tm=squadov_common::sql_format_time(&m.tm),
                entity_id=m.real_entity_id.unwrap_or(0),
                tags=squadov_common::sql_format_json(&m.tags)?,
                attributes=squadov_common::sql_format_json(&m.attributes)?,
                parent_block=squadov_common::sql_format_option_string(&m.current_block_id),
                action_type=m.action_type as i32,
            ));

            if idx != actions.len() - 1 {
                sql.push(String::from(","));
            }
        }

        sqlx::query(&sql.join("")).execute(tx).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_snapshots(&self, tx: &mut Transaction<'_, Postgres> , snapshots: &[HearthstoneGameSnapshot], uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        if snapshots.is_empty() {
            return Ok(());
        }
        // Each snapshot contains information that needs to be inserted into 3 different tables:
        // 1) hearthstone_snapshots: Contains general information about the snapshot (time, turn, etc)
        // 2) hearthstone_snapshots_player_map: Contains information about what we know about the player name -> player id -> entity id map at that point in time.
        // 3) hearthstone_snapshots_entities: Contains information about what we know about all the entities on the board at that time.
        let mut snapshots_sql : Vec<String> = Vec::new();
        snapshots_sql.push(String::from("
            INSERT INTO squadov.hearthstone_snapshots (
                snapshot_id,
                match_uuid,
                user_id,
                last_action_id,
                tm,
                game_entity_id,
                current_turn,
                step,
                current_player_id
            )
            VALUES
        "));

        let mut players_sql : Vec<String> = Vec::new();
        players_sql.push(String::from("
            INSERT INTO squadov.hearthstone_snapshots_player_map (
                snapshot_id,
                player_name,
                player_id,
                entity_id
            )
            VALUES
        "));

        let mut entities_sql : Vec<String> = Vec::new();
        entities_sql.push(String::from("
            INSERT INTO squadov.hearthstone_snapshots_entities (
                snapshot_id,
                entity_id,
                tags,
                attributes
            )
            VALUES
        "));

        for m in snapshots {
            let aux = m.aux_data.as_ref().unwrap();
            snapshots_sql.push(format!("(
                '{snapshot_id}',
                '{match_uuid}',
                {user_id},
                {last_action_id},
                {tm},
                {game_entity_id},
                {current_turn},
                {step},
                {current_player_id}
            )",
                snapshot_id=m.uuid,
                match_uuid=uuid,
                user_id=user_id,
                last_action_id=aux.last_action_index,
                tm=squadov_common::sql_format_option_some_time(m.tm.as_ref()),
                game_entity_id=m.game_entity_id,
                current_turn=aux.current_turn,
                step=aux.step as i32,
                current_player_id=aux.current_player_id
            ));
            snapshots_sql.push(String::from(","));

            // Going to assume that player_name_to_player_id hashmap is the same size as the
            // player_id_to_entity_id hashmap. Pretty sure that this is a safe assumption.
            for (name, pid) in &m.player_name_to_player_id {
                let eid = m.player_id_to_entity_id.get(pid).unwrap_or(&0);
                players_sql.push(format!("(
                    '{snapshot_id}',
                    '{player_name}',
                    {player_id},
                    {entity_id}
                )",
                    snapshot_id=m.uuid,
                    player_name=&name,
                    player_id=pid,
                    entity_id=eid
                ));
                players_sql.push(String::from(","));
            }

            for (eid, entity) in &m.entities {
                entities_sql.push(format!("(
                    '{snapshot_id}',
                    {entity_id},
                    '{tags}',
                    '{attributes}'
                )",
                    snapshot_id=m.uuid,
                    entity_id=eid,
                    tags=squadov_common::sql_format_json(&entity.tags)?,
                    attributes=squadov_common::sql_format_json(&entity.attributes)?
                ));
                entities_sql.push(String::from(","));
            }
        }

        snapshots_sql.truncate(snapshots_sql.len() - 1);
        players_sql.truncate(players_sql.len() - 1);
        entities_sql.truncate(entities_sql.len() - 1);

        tx.execute(sqlx::query(&snapshots_sql.join(""))).await?;
        tx.execute(sqlx::query(&players_sql.join(""))).await?;
        tx.execute(sqlx::query(&entities_sql.join(""))).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_blocks(&self, tx: &mut Transaction<'_, Postgres> , blocks: &HashMap<Uuid, HearthstoneGameBlock>, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        if blocks.is_empty() {
            return Ok(());
        }

        // Each snapshot contains information that needs to be inserted into 3 different tables:
        // 1) hearthstone_snapshots: Contains general information about the snapshot (time, turn, etc)
        // 2) hearthstone_snapshots_player_map: Contains information about what we know about the player name -> player id -> entity id map at that point in time.
        // 3) hearthstone_snapshots_entities: Contains information about what we know about all the entities on the board at that time.
        let mut sql : Vec<String> = Vec::new();
        sql.push("
            INSERT INTO squadov.hearthstone_blocks(
                match_uuid,
                user_id,
                block_id,
                start_action_index,
                end_action_index,
                block_type,
                parent_block,
                entity_id
            )
            VALUES
        ".to_string());

        for (_, block) in blocks {
            sql.push(format!("(
                '{match_uuid}',
                {user_id},
                '{block_id}',
                {start_action_index},
                {end_action_index},
                {block_type},
                {parent_block},
                {entity_id}
            )",
                match_uuid=uuid,
                user_id=user_id,
                block_id=block.block_id,
                start_action_index=block.start_action_index,
                end_action_index=block.end_action_index,
                block_type=block.block_type as i32,
                parent_block=squadov_common::sql_format_option_string(&block.parent_block),
                entity_id=block.entity_id
            ));
            sql.push(", ".to_string());
        }

        sql.truncate(sql.len() - 1);
        tx.execute(sqlx::query(&sql.join(""))).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_log(&self, tx: &mut Transaction<'_, Postgres> , logs: &HearthstoneGameLog, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        self.store_hearthstone_match_game_blocks(tx, &logs.blocks, uuid, user_id).await?;
        self.store_hearthstone_match_game_actions(tx, &logs.actions, uuid, user_id).await?;
        self.store_hearthstone_match_game_snapshots(tx, &logs.snapshots, uuid, user_id).await?;
        tx.execute(
            sqlx::query!(
                "
                UPDATE squadov.hearthstone_raw_power_logs
                SET parsed = TRUE
                WHERE match_uuid = $1
                    AND user_id = $2
                ",
                uuid,
                user_id
            )
        ).await?;
        Ok(())
    }

    pub async fn associate_hearthstone_match_with_arena_run(&self, tx: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        // Note that each hearthstone match could be associated with 2 arena runs - one for each player.
        sqlx::query!(
            "
            INSERT INTO squadov.match_to_match_collection (
                match_uuid,
                collection_uuid
            )
            SELECT $1, had.collection_uuid
            FROM squadov.hearthstone_arena_drafts AS had
            INNER JOIN squadov.hearthstone_decks AS hpmd
                ON hpmd.deck_id = had.draft_deck_id AND hpmd.user_id = had.user_id
            WHERE had.user_id = $2
            ",
            match_uuid,
            user_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }
}

pub async fn create_hearthstone_match_handler(data : web::Json<CreateHearthstoneMatchInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_hearthstone_match(&mut tx, &data.timestamp, &data.info).await?;
    match &data.deck {
        Some(x) => {
            app.store_hearthstone_deck(&mut tx, &x, session.user.id).await?;
            app.associate_deck_with_match_user(&mut tx, x.deck_id, &uuid, session.user.id).await?;
        },
        None => (),
    };

    // Handle each player separately instead of batching for ease of us. There's only 2 players in any
    // given call anyway so it's not too expensive.
    for (player_id, player) in &data.players {
        app.store_hearthstone_match_player(&mut tx, *player_id, &player, &uuid, session.user.id).await?;
    }
    
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(&uuid))
}

pub async fn upload_hearthstone_logs_handler(data : web::Json<Vec<hearthstone::HearthstoneRawLog>>, path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    app.store_hearthstone_raw_power_logs(&mut tx, &data, &path.match_uuid, session.user.id).await?;
    tx.commit().await?;

    // Handle the parsed logs in a separate transaction so that we have the raw logs even if this parsing fails.
    // Idealy we'd throw this onto a thread pool via Tokio or something.
    let mut tx = app.pool.begin().await?;
    let mut parser = HearthstonePowerLogParser::new(false);
    parser.parse(&data)?;

    app.store_hearthstone_match_metadata(&mut tx, &parser.state, &path.match_uuid).await?;

    // If the match is for the Arena then we must associate the match with the right match collection
    // using the stored deck ID.
    if parser.state.game_type == GameType::Arena {
        app.associate_hearthstone_match_with_arena_run(&mut tx, &path.match_uuid, session.user.id).await?;
    }

    app.store_hearthstone_match_game_log(&mut tx, &parser.fsm.game.borrow(), &path.match_uuid, session.user.id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().finish())
}