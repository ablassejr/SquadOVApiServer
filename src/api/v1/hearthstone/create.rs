use crate::common;
use crate::common::hearthstone;
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
    pub async fn check_if_hearthstone_match_exists(&self, timestamp: &DateTime<Utc>, info: &hearthstone::HearthstoneGameConnectionInfo) -> Result<Option<v1::Match>, common::SquadOvError> {
        Ok(sqlx::query_as!(
            v1::Match,
            "
            SELECT match_uuid as \"uuid\"
            FROM squadov.hearthstone_matches
            WHERE match_day = $1
                AND server_ip = $2
                AND port = $3
                AND game_id = $4
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
    pub async fn create_hearthstone_match(&self, tx : &mut Transaction<'_, Postgres>, timestamp: &DateTime<Utc>, info: &hearthstone::HearthstoneGameConnectionInfo) -> Result<Uuid, common::SquadOvError> {
        // Check if a match already exists as we want to be as close to storing 1 entry per actual server-side match as possible.
        let current_match = match self.check_if_hearthstone_match_exists(timestamp, info).await? {
            Some(x) => x,
            None => {
                // A reconnecting game should already exist in our database.
                if info.reconnecting {
                    return Err(common::SquadOvError::NotFound);
                } else {
                    let mt = self.create_new_match(tx).await?;
                    sqlx::query!(
                        "
                        INSERT INTO squadov.hearthstone_matches (
                            match_uuid,
                            server_ip,
                            port,
                            game_id,
                            match_day
                        )
                        VALUES (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5
                        )
                        ",
                        mt.uuid,
                        info.ip,
                        info.port,
                        info.game_id,
                        timestamp.date().naive_utc(),
                    )
                        .execute(tx)
                        .await?;
                    mt
                }
            }
        };

        Ok(current_match.uuid)
    }

    pub async fn store_hearthstone_deck_slots(&self, tx : &mut Transaction<'_, Postgres>, match_uuid: &Uuid, deck_id: i64, slots: &[hearthstone::HearthstoneDeckSlot]) -> Result<(), common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.hearthstone_player_match_deck_slots (
                match_uuid,
                deck_id,
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in slots.iter().enumerate() {
            sql.push(format!("(
                '{match_uuid}',
                {deck_id},
                {index},
                '{card_id}',
                {owned},
                {normal_count},
                {golden_count}
            )",
                match_uuid=match_uuid,
                deck_id=deck_id,
                index=m.index,
                card_id=&m.card_id,
                owned=m.owned,
                normal_count=m.count.normal,
                golden_count=m.count.golden,
            ));

            if idx != slots.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }

        sql.push(String::from(" ON CONFLICT DO NOTHING"));
        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    pub async fn store_hearthstone_deck_for_user_match(&self, tx : &mut Transaction<'_, Postgres>, deck: &hearthstone::HearthstoneDeck, match_uuid: &Uuid, user_id: i64) -> Result<(), common::SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.hearthstone_player_match_decks (
                    user_id,
                    match_uuid,
                    deck_id,
                    deck_name,
                    hero_card,
                    hero_premium,
                    deck_type,
                    create_date,
                    is_wild
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
                user_id,
                match_uuid,
                deck.deck_id,
                deck.name,
                deck.hero_card,
                deck.hero_premium,
                deck.deck_type,
                deck.create_date,
                deck.is_wild
            )
        ).await?;

        self.store_hearthstone_deck_slots(tx, match_uuid, deck.deck_id, &deck.slots).await?;
        Ok(())
    }

    pub async fn store_hearthstone_player_medal_info(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, match_uuid: &Uuid, info: &hearthstone::HearthstoneMedalInfo, is_standard: bool) -> Result<(), common::SquadOvError> {
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

    pub async fn store_hearthstone_match_player(&self, tx : &mut Transaction<'_, Postgres>, player_match_id: i32, player: &hearthstone::HearthstonePlayer, match_uuid: &Uuid, user_id: i64) -> Result<(), common::SquadOvError> {
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

    pub async fn store_hearthstone_raw_power_logs(&self, tx : &mut Transaction<'_, Postgres>, logs: &[hearthstone::HearthstoneRawLog], match_uuid: &Uuid, user_id: i64) -> Result<(), common::SquadOvError> {
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
}

pub async fn create_hearthstone_match_handler(data : web::Json<CreateHearthstoneMatchInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(common::SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_hearthstone_match(&mut tx, &data.timestamp, &data.info).await?;
    match &data.deck {
        Some(x) => app.store_hearthstone_deck_for_user_match(&mut tx, &x, &uuid, session.user.id).await?,
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

pub async fn upload_hearthstone_logs_handler(data : web::Json<Vec<hearthstone::HearthstoneRawLog>>, path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(common::SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    app.store_hearthstone_raw_power_logs(&mut tx, &data, &path.match_uuid, session.user.id).await?;
    tx.commit().await?;

    // Process power logs on a separate thread. Ideally we'd probably want to
    // throw this onto a RabbitMQ queue and handle the parsing elsewhere so we can
    // more accurately control the amount of resources used to parse logs.

    Ok(HttpResponse::Ok().finish())
}