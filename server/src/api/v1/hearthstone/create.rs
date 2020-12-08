use squadov_common;
use squadov_common::hearthstone;
use squadov_common::hearthstone::{HearthstoneRawLog, power_parser::{HearthstonePowerLogParser, HearthstoneGameState} };
use squadov_common::hearthstone::game_state::{HearthstoneGameLog};
use squadov_common::hearthstone::GameType;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use sqlx::{Transaction, Executor, Postgres};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use futures_util::StreamExt;
use chrono::{DateTime, Utc};
use std::io::Read;
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

    pub async fn create_hearthstone_match_view(&self,  tx : &mut Transaction<'_, Postgres>, match_uuid: &Uuid, user_id: i64) -> Result<Uuid, squadov_common::SquadOvError> {
        let view_uuid = Uuid::new_v4();
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_view (
                view_uuid,
                match_uuid,
                user_id
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ",
            &view_uuid,
            match_uuid,
            user_id,
        )
            .execute(tx)
            .await?;
        Ok(view_uuid)
    }

    pub async fn store_hearthstone_player_medal_info(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, view_uuid: &Uuid, info: &hearthstone::HearthstoneMedalInfo, is_standard: bool) -> Result<(), squadov_common::SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_player_medals (
                view_uuid,
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
            view_uuid,
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

    pub async fn store_hearthstone_match_player(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, player: &hearthstone::HearthstonePlayer, view_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.hearthstone_match_players (
                    user_id,
                    view_uuid,
                    player_match_id,
                    player_name,
                    card_back_id,
                    arena_wins,
                    arena_loss,
                    tavern_brawl_wins,
                    tavern_brawl_loss,
                    battlegrounds_rating,
                    duels_casual_rating,
                    duels_heroic_rating
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
                    $11,
                    $12
                )
                ON CONFLICT (view_uuid, player_match_id) DO UPDATE SET
                    user_id = COALESCE(squadov.hearthstone_match_players.user_id, EXCLUDED.user_id),
                    arena_wins = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.arena_wins
                                      ELSE squadov.hearthstone_match_players.arena_wins END,
                    arena_loss = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.arena_loss
                                 ELSE squadov.hearthstone_match_players.arena_loss END,
                    tavern_brawl_wins = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.tavern_brawl_wins
                                        ELSE squadov.hearthstone_match_players.tavern_brawl_wins END,
                    tavern_brawl_loss = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.tavern_brawl_loss
                                        ELSE squadov.hearthstone_match_players.tavern_brawl_loss END,
                    battlegrounds_rating = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.battlegrounds_rating
                                           ELSE squadov.hearthstone_match_players.battlegrounds_rating END,
                    duels_casual_rating = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.duels_casual_rating
                                          ELSE squadov.hearthstone_match_players.duels_casual_rating END,
                    duels_heroic_rating = CASE WHEN EXCLUDED.user_id IS NOT NULL THEN EXCLUDED.duels_heroic_rating
                                          ELSE squadov.hearthstone_match_players.duels_heroic_rating END
                ",
                // *ONLY* The local player should be associated with the user we pulled from the session.
                // Ideally we'd get them to OAuth with Blizzard to verify account ownership instead?
                if player.local {
                    Some(user_id)
                } else {
                    None
                },
                view_uuid,
                player_match_id,
                player.name,
                player.card_back_id,
                player.arena_wins as i32,
                player.arena_loss as i32,
                player.tavern_brawl_wins as i32,
                player.tavern_brawl_loss as i32,
                player.battlegrounds_rating,
                player.duels_casual_rating,
                player.duels_heroic_rating
            )
        ).await?;

        self.store_hearthstone_player_medal_info(tx, player_match_id, view_uuid, &player.medal_info.standard, true).await?;
        self.store_hearthstone_player_medal_info(tx, player_match_id, view_uuid, &player.medal_info.wild, false).await?;
        Ok(())
    }

    pub async fn store_hearthstone_raw_power_logs(&self, tx : &mut Transaction<'_, Postgres>, data: &[u8], match_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        let blob_uuid = self.blob.store_new_blob(tx, data).await?;
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.hearthstone_raw_power_logs (
                    user_id,
                    match_uuid,
                    parsed,
                    raw_logs_blob_uuid
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
                false,
                blob_uuid
            )
        ).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_metadata(&self, tx : &mut Transaction<'_, Postgres>, st: &HearthstoneGameState, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_metadata (
                view_uuid,
                game_type,
                format_type,
                scenario_id,
                match_winner_player_id
            )
            SELECT
                hmv.view_uuid,
                $3,
                $4,
                $5,
                $6
            FROM squadov.hearthstone_match_view AS hmv
            WHERE hmv.match_uuid = $1 AND hmv.user_id = $2
            ",
            uuid,
            user_id,
            st.game_type as i32,
            st.format_type as i32,
            st.scenario_id,
            st.match_winner_player_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_actions(&self, tx: &mut Transaction<'_, Postgres>, logs: Arc<RwLock<HearthstoneGameLog>>, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        let raw_data;

        {
            let actions = &logs.read()?.actions;
            if actions.is_empty() {
                return Ok(());
            }
            raw_data = serde_json::to_value(actions)?;
        }

        let blob_uuid = self.blob.store_new_json_blob(tx, &raw_data).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_match_action_blobs (
                match_uuid,
                user_id,
                actions_blob_uuid
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ",
            uuid,
            user_id,
            blob_uuid
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_snapshots(&self, tx: &mut Transaction<'_, Postgres> , logs: Arc<RwLock<HearthstoneGameLog>>, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        // Each snapshot contains information that needs to be inserted into 3 different tables:
        // 1) hearthstone_snapshots: Contains general information about the snapshot (time, turn, etc)
        // 2) hearthstone_snapshots_player_map: Contains information about what we know about the player name -> player id -> entity id map at that point in time.
        // 3) hearthstone_snapshots_entities: Contains information about what we know about all the entities on the board at that time.
        let mut snapshots_sql : Vec<String> = Vec::new();
        let mut players_sql : Vec<String> = Vec::new();
        let mut entities_sql : Vec<String> = Vec::new();

        {
            let snapshots = &logs.read()?.snapshots;
            if snapshots.is_empty() {
                return Ok(());
            }
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

            players_sql.push(String::from("
                INSERT INTO squadov.hearthstone_snapshots_player_map (
                    snapshot_id,
                    player_name,
                    player_id,
                    entity_id
                )
                VALUES
            "));

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
        }

        snapshots_sql.truncate(snapshots_sql.len() - 1);
        players_sql.truncate(players_sql.len() - 1);
        entities_sql.truncate(entities_sql.len() - 1);

        tx.execute(sqlx::query(&snapshots_sql.join(""))).await?;
        tx.execute(sqlx::query(&players_sql.join(""))).await?;
        tx.execute(sqlx::query(&entities_sql.join(""))).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_blocks(&self, tx: &mut Transaction<'_, Postgres> , logs: Arc<RwLock<HearthstoneGameLog>>, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        // Each snapshot contains information that needs to be inserted into 3 different tables:
        // 1) hearthstone_snapshots: Contains general information about the snapshot (time, turn, etc)
        // 2) hearthstone_snapshots_player_map: Contains information about what we know about the player name -> player id -> entity id map at that point in time.
        // 3) hearthstone_snapshots_entities: Contains information about what we know about all the entities on the board at that time.
        let mut sql : Vec<String> = Vec::new();

        {
            let blocks = &logs.read()?.blocks;
            if blocks.is_empty() {
                return Ok(());
            }

            
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
        }

        sql.truncate(sql.len() - 1);
        tx.execute(sqlx::query(&sql.join(""))).await?;
        Ok(())
    }

    pub async fn store_hearthstone_match_game_log(&self, tx: &mut Transaction<'_, Postgres> , logs: Arc<RwLock<HearthstoneGameLog>>, uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        self.store_hearthstone_match_game_blocks(tx, logs.clone(), uuid, user_id).await?;
        self.store_hearthstone_match_game_actions(tx, logs.clone(), uuid, user_id).await?;
        self.store_hearthstone_match_game_snapshots(tx, logs.clone(), uuid, user_id).await?;
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

    pub async fn associate_hearthstone_match_with_duels_run(&self, tx: &mut Transaction<'_, Postgres> , match_uuid: &Uuid, user_id: i64, start_time: &DateTime<Utc>) -> Result<(), squadov_common::SquadOvError> {
        // First check if a duels run already exists.
        let mut duels_run_uuid = sqlx::query_scalar(
            "
            SELECT hd.collection_uuid
            FROM squadov.hearthstone_duels AS hd
            INNER JOIN squadov.hearthstone_deck_versions AS hdv
                ON hdv.deck_id = hd.deck_id
            INNER JOIN squadov.hearthstone_match_user_deck AS hmud
                ON hmud.deck_version_id = hdv.version_id
            WHERE hd.user_id = $2 AND hmud.user_id = $2 AND hmud.match_uuid = $1
            "
        )
            .bind(match_uuid)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await?;

        if duels_run_uuid.is_none() {
            duels_run_uuid = Some(self.create_hearthstone_duels_run(tx, match_uuid, user_id, start_time).await?);
        }

        let duels_run_uuid = duels_run_uuid.unwrap();
        sqlx::query!(
            "
            INSERT INTO squadov.match_to_match_collection (
                match_uuid,
                collection_uuid
            )
            VALUES (
                $1,
                $2
            )
            ",
            match_uuid,
            duels_run_uuid,
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn parse_hearthstone_power_logs(&self, data: &[u8], match_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        // Need to try to uncompress using GZIP. If that fails we'll assume that the input data is raw JSON data.
        // TODO: Use the HTTP headers instead?
        let mut gz = flate2::read::GzDecoder::new(data);
        let mut uncompressed_data: Vec<u8> = Vec::new();
        gz.read_to_end(&mut uncompressed_data)?;
        let data: Vec<HearthstoneRawLog> = match gz.read_to_end(&mut uncompressed_data) {
            Ok(_) => serde_json::from_slice(&uncompressed_data)?,
            Err(_) => serde_json::from_slice(&data)? 
        };

        let mut tx = self.pool.begin().await?;
        let parser = Arc::new(RwLock::new(HearthstonePowerLogParser::new(false)));

        // This needs to be a match otherwise Rustc gives us an type annotations needed error?
        match parser.write()?.parse(&data) {
            Ok(_) => (),
            Err(e) => return Err(e)
        };

        {
            let state = parser.read()?.state.clone();
            self.store_hearthstone_match_metadata(&mut tx, &state, match_uuid, user_id).await?;
        }

        {
            // If the match is for the Arena then we must associate the match with the right match collection
            // using the stored deck ID. If the match is for Duels then we must either associate the match with
            // an already created Duels run or create a new match collection using the current deck's ID.
            let game_type = parser.read()?.state.game_type;
            if  game_type == GameType::Arena {
                self.associate_hearthstone_match_with_arena_run(&mut tx, match_uuid, user_id).await?;
            } else if game_type == GameType::PvpDr || game_type == GameType::PvpDrPaid {
                let start_time = parser.read()?.get_log_start_time();
                self.associate_hearthstone_match_with_duels_run(&mut tx, match_uuid, user_id, &start_time).await?;
            }
        }

        {
            let game = parser.read()?.fsm.game.clone();
            self.store_hearthstone_match_game_log(&mut tx, game, match_uuid, user_id).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Deserialize)]
pub struct CreateHearthstoneMatchPathInput {
    user_id: i64
}

pub async fn create_hearthstone_match_handler(data : web::Json<CreateHearthstoneMatchInput>, app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CreateHearthstoneMatchPathInput>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let mut tx = app.pool.begin().await?;
    let match_uuid = app.create_hearthstone_match(&mut tx, &data.timestamp, &data.info).await?;

    // Need to also create a VIEW associated with this match. As the data is collected by each user locally
    // and not by a singular central source, we have to assume that each view may have some conflicting data.
    // Thus, all data is generally associated with a match view and we use the match only as a way to allow
    // users to see different views (assuming they have permissions to do so).
    let view_uuid = app.create_hearthstone_match_view(&mut tx, &match_uuid, path.user_id).await?;

    match &data.deck {
        Some(x) => {
            // store_hearthstone_deck MUST be called before associate_deck_with_match_user because
            // associate_deck_with_match_user assumes that the latest deck is the correct deck version
            // to associate with the match. So if associate_deck_with_match_user is called before
            // store_hearthstone_deck, it'll be possible for us to associate an older version of the deck
            // with the match incorrectly.
            app.store_hearthstone_deck(&mut tx, &x, path.user_id).await?;
            app.associate_deck_with_match_user(&mut tx, x.deck_id, &match_uuid, path.user_id).await?;
        },
        None => (),
    };

    // Handle each player separately instead of batching for ease of us. There's only 2 players in any
    // given call anyway so it's not too expensive.
    for (player_id, player) in &data.players {
        app.store_hearthstone_match_player(&mut tx, *player_id, &player, &view_uuid, path.user_id).await?;
    }
    
    tx.commit().await?;

    // We don't need to send the view_uuid back to the user since the combination of the match_uuid
    // along with the given user id is enough to identify the view.
    Ok(HttpResponse::Ok().json(&match_uuid))
}

// Note that we don't parse directly into the expected data structures immediately as that can happen in an async thread so we can return to the user faster.
pub async fn upload_hearthstone_logs_handler(mut body : web::Payload, path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let user_id = path.user_id;

    // This grabs the raw byte data (it may be compressed or uncompressed!).
    // Uncompressed this could be really large (and thus take awhile to uncompress)
    // so we uncompress in a separate thread so that the user doesn't have to wait
    // for that to happen.
    let mut data = web::BytesMut::new();
    while let Some(item) = body.next().await {
        data.extend_from_slice(&item?);
    }

    let app = app.clone();
    {
        let mut tx = app.pool.begin().await?;
        app.store_hearthstone_raw_power_logs(&mut tx, &data, &path.match_uuid, user_id).await?;
        tx.commit().await?;
    }

    // Do the log parsing in a separate thread because it's potentially a fairly lengthy process.
    tokio::task::spawn(async move {
        match app.parse_hearthstone_power_logs(&data, &path.match_uuid, user_id).await {
            Ok(_) => Ok(()),
            Err(err) => {
                log::error!("Failed to parse Hearthstone logs: {:?}", err);
                Err(err)
            }
        }
    });

    Ok(HttpResponse::Ok().finish())
}