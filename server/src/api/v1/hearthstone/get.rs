use squadov_common::SquadOvError;
use squadov_common::hearthstone::{HearthstoneDeck, HearthstoneDeckSlot, HearthstoneCardCount, HearthstonePlayer, HearthstonePlayerMedalInfo, FormatType, GameType};
use squadov_common::hearthstone::game_state::{HearthstoneGameBlock, HearthstoneGameSnapshot, HearthstoneGameSnapshotAuxData, HearthstoneGameAction, HearthstoneEntity, game_step::GameStep, EntityId};
use squadov_common::hearthstone::game_packet::{HearthstoneMatchMetadata, HearthstoneGamePacket, HearthstoneGameLogMetadata, HearthstoneSerializedGameLog};
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::auth::SquadOVSession;
use std::convert::TryFrom;
use std::collections::HashMap;

impl api::ApiApplication {

    pub async fn get_hearthstone_snapshot(&self, snapshot_uuid: &Uuid) -> Result<HearthstoneGameSnapshot, SquadOvError> {
        let raw_metadata = sqlx::query!(
            "
            SELECT
                tm,
                game_entity_id,
                current_turn,
                step,
                current_player_id,
                last_action_id
            FROM squadov.hearthstone_snapshots
            WHERE snapshot_id = $1
            ",
            snapshot_uuid
        )
            .fetch_one(&*self.pool)
            .await?;

        let mut snapshot = HearthstoneGameSnapshot::new();
        snapshot.uuid = snapshot_uuid.clone();
        snapshot.tm = raw_metadata.tm;
        snapshot.game_entity_id = raw_metadata.game_entity_id;
        snapshot.aux_data = Some(HearthstoneGameSnapshotAuxData{
            current_turn: raw_metadata.current_turn,
            step: GameStep::try_from(raw_metadata.step)?,
            current_player_id: raw_metadata.current_player_id,
            last_action_index: raw_metadata.last_action_id as usize
        });

        let snapshot_players = sqlx::query!(
            "
            SELECT 
                player_name,
                player_id,
                entity_id
            FROM squadov.hearthstone_snapshots_player_map
            WHERE snapshot_id = $1
            ",
            snapshot_uuid
        )
            .fetch_all(&*self.pool)
            .await?;

        for sp in snapshot_players {
            snapshot.player_name_to_player_id.insert(sp.player_name, sp.player_id);
            snapshot.player_id_to_entity_id.insert(sp.player_id, sp.entity_id);
        }

        let snapshot_entities = sqlx::query!(
            "
            SELECT
                entity_id,
                tags,
                attributes
            FROM squadov.hearthstone_snapshots_entities
            WHERE snapshot_id = $1
            ",
            snapshot_uuid
        )
            .fetch_all(&*self.pool)
            .await?;

        for se in snapshot_entities {
            let entity = HearthstoneEntity{
                entity_id: se.entity_id,
                tags: serde_json::from_value(se.tags)?,
                attributes: serde_json::from_value(se.attributes)?
            };

            snapshot.entities.insert(se.entity_id, entity);
        }

        Ok(snapshot)
    }

    pub async fn get_hearthstone_deck_for_match_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<Option<HearthstoneDeck>, SquadOvError> {
        let raw_deck = sqlx::query!(
            "
            SELECT
                deck_id,
                hero_card,
                hero_premium,
                deck_type,
                create_date,
                is_wild,
                deck_name AS \"name\"
            FROM squadov.hearthstone_player_match_decks
            WHERE match_uuid = $1
                AND user_id = $2
            ",
            match_uuid,
            user_id
        )
            .fetch_optional(&*self.pool)
            .await?;

        if raw_deck.is_none() {
            return Ok(None);
        }

        let raw_deck = raw_deck.unwrap();
        let raw_slots = sqlx::query!(
            "
            SELECT
                index,
                card_id,
                owned,
                normal_count,
                golden_count
            FROM squadov.hearthstone_player_match_deck_slots
            WHERE match_uuid = $1
                AND deck_id = $2
            ",
            match_uuid,
            raw_deck.deck_id
        )
            .fetch_all(&*self.pool)
            .await?;

        Ok(Some(HearthstoneDeck{
            slots: raw_slots.into_iter().map(|x| {
                HearthstoneDeckSlot {
                    index: x.index,
                    card_id: x.card_id,
                    owned: x.owned,
                    count: HearthstoneCardCount{
                        normal: x.normal_count,
                        golden: x.golden_count
                    },
                }
            }).collect(),
            name: raw_deck.name,
            deck_id: raw_deck.deck_id,
            hero_card: raw_deck.hero_card,
            hero_premium: raw_deck.hero_premium,
            deck_type: raw_deck.deck_type,
            create_date: raw_deck.create_date,
            is_wild: raw_deck.is_wild
        }))
    }

    pub async fn get_hearthstone_players_for_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<HashMap<i32, HearthstonePlayer>, SquadOvError> {
        let raw_match_players = sqlx::query!(
            "
            SELECT
                user_id,
                player_match_id,
                player_name,
                card_back_id,
                arena_wins,
                arena_loss,
                tavern_brawl_wins,
                tavern_brawl_loss
            FROM squadov.hearthstone_match_players
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(&*self.pool)
            .await?;

        let raw_match_player_medals = sqlx::query!(
            "
            SELECT
                player_match_id,
                league_id,
                earned_stars,
                star_level,
                best_star_level,
                win_streak,
                legend_index,
                is_standard
            FROM squadov.hearthstone_match_player_medals
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret_map: HashMap<i32, HearthstonePlayer> = HashMap::new();
        for rmp in raw_match_players {
            let new_player = HearthstonePlayer{
                name: rmp.player_name,
                local: rmp.user_id.unwrap_or(-1) == user_id,
                side: 0,
                card_back_id: rmp.card_back_id,
                arena_wins: rmp.arena_wins as u32,
                arena_loss: rmp.arena_loss as u32,
                tavern_brawl_wins: rmp.tavern_brawl_wins as u32,
                tavern_brawl_loss: rmp.tavern_brawl_loss as u32,
                medal_info: HearthstonePlayerMedalInfo::new(),
            };

            ret_map.insert(rmp.player_match_id, new_player);
        }

        for medal in raw_match_player_medals {
            let player = ret_map.get_mut(&medal.player_match_id);
            if player.is_none() {
                continue;
            }
            let player = player.unwrap();
            let medal_info = if medal.is_standard {
                &mut player.medal_info.standard
            } else {
                &mut player.medal_info.wild
            };

            medal_info.league_id = medal.league_id;
            medal_info.earned_stars = medal.earned_stars;
            medal_info.star_level = medal.star_level;
            medal_info.best_star_level = medal.best_star_level;
            medal_info.win_streak = medal.win_streak;
            medal_info.legend_index = medal.legend_index;
        }

        Ok(ret_map)
    }

    pub async fn get_hearthstone_match_for_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<HearthstoneGamePacket, SquadOvError> {
        // Give users some summary data about the match and then just dump the latest snapshot on them and let them figure out what
        // data they need from the snapshot on their own.
        let raw_metadata = sqlx::query!(
            "
            SELECT
                hmm.game_type,
                hmm.format_type,
                hmm.scenario_id,
                hm.match_time,
                COALESCE(EXTRACT(EPOCH FROM (
                    SELECT tm
                    FROM squadov.hearthstone_actions
                    ORDER BY action_id DESC
                    LIMIT 1
                )) - EXTRACT(EPOCH FROM (
                    SELECT tm
                    FROM squadov.hearthstone_actions
                    ORDER BY action_id ASC
                    LIMIT 1
                )), 0) AS \"elapsed_seconds!\"
            FROM squadov.hearthstone_matches AS hm
            INNER JOIN squadov.hearthstone_match_metadata AS hmm
                ON hmm.match_uuid = hm.match_uuid
            WHERE hm.match_uuid = $1
            ",
            match_uuid
        )
            .fetch_one(&*self.pool)
            .await?;

        let metadata = HearthstoneMatchMetadata{
            game_type: GameType::try_from(raw_metadata.game_type)?,
            format_type: FormatType::try_from(raw_metadata.format_type)?,
            scenario_id: raw_metadata.scenario_id,
            match_time: raw_metadata.match_time,
            elapsed_seconds: raw_metadata.elapsed_seconds,
            deck: self.get_hearthstone_deck_for_match_user(match_uuid, user_id).await?,
            players: self.get_hearthstone_players_for_match(match_uuid, user_id).await?
        };
        
        // Is there a way to combine this into 1 SQL statement?
        let log_metadata = HearthstoneGameLogMetadata{
            snapshot_ids: sqlx::query_scalar(
                "
                SELECT hs.snapshot_id
                FROM squadov.hearthstone_snapshots AS hs
                WHERE hs.match_uuid = $1 AND hs.user_id = $2
                ORDER BY hs.last_action_id ASC
                ",
            )
                .bind(match_uuid)
                .bind(user_id)
                .fetch_all(&*self.pool)
                .await?,
            num_actions: sqlx::query_scalar(
                "
                SELECT COUNT(action_id)
                FROM squadov.hearthstone_actions AS ha
                WHERE ha.match_uuid = $1 and ha.user_id = $2
                ",
            )
                .bind(match_uuid)
                .bind(user_id)
                .fetch_one(&*self.pool)
                .await?
        };

        let latest_snapshot = match log_metadata.snapshot_ids.last() {
            Some(x) => Some(self.get_hearthstone_snapshot(x).await?),
            None => None
        };

        // Also retrieve the last snapshot of the game and just send it to the user "raw" and let them
        // parse it to retrieve some data about the final state of the game (e.g. this is what we're going
        // to use to display the final board state of the game).
        Ok(HearthstoneGamePacket{
            match_uuid: match_uuid.clone(),
            metadata: metadata,
            log_metadata: log_metadata,
            latest_snapshot: latest_snapshot
        })
    }

    pub async fn get_hearthstone_match_logs_for_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<HearthstoneSerializedGameLog, SquadOvError> {
        let snapshot_ids: Vec<Uuid> = sqlx::query_scalar(
            "
            SELECT hs.snapshot_id
            FROM squadov.hearthstone_snapshots AS hs
            WHERE hs.match_uuid = $1 AND hs.user_id = $2
            ORDER BY hs.last_action_id ASC
            ",
        )
            .bind(match_uuid)
            .bind(user_id)
            .fetch_all(&*self.pool)
            .await?;

        let mut snapshots: Vec<HearthstoneGameSnapshot> = Vec::new();
        for id in snapshot_ids {
            snapshots.push(self.get_hearthstone_snapshot(&id).await?);
        }

        let raw_actions = sqlx::query!(
            "
            SELECT
                action_id,
                tm,
                entity_id,
                tags,
                attributes,
                parent_block
            FROM squadov.hearthstone_actions
            WHERE match_uuid = $1
                AND user_id = $2
            ",
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;
        let mut actions : Vec<HearthstoneGameAction> = Vec::new();
        for ra in raw_actions {
            actions.push(HearthstoneGameAction{
                tm: ra.tm,
                entity_id: EntityId::None,
                current_block_id: ra.parent_block,
                real_entity_id: Some(ra.entity_id),
                tags: serde_json::from_value(ra.tags)?,
                attributes: serde_json::from_value(ra.attributes)?
            });
        }

        Ok(HearthstoneSerializedGameLog{
            snapshots,
            actions,
            blocks: sqlx::query_as::<_,HearthstoneGameBlock>(
                "
                SELECT 
                    block_id,
                    start_action_index,
                    end_action_index,
                    block_type,
                    parent_block
                FROM squadov.hearthstone_blocks
                WHERE match_uuid = $1
                    AND user_id = $2
                ",
            )
                .bind(match_uuid)
                .bind(user_id)
                .fetch_all(&*self.pool)
                .await?
        })
    }
}

pub async fn get_hearthstone_match_handler(path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    let packet = app.get_hearthstone_match_for_user(&path.match_uuid, session.user.id).await?;
    Ok(HttpResponse::Ok().json(&packet))
}

pub async fn get_hearthstone_match_logs_handler(path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    let logs = app.get_hearthstone_match_logs_for_user(&path.match_uuid, session.user.id).await?;
    Ok(HttpResponse::Ok().json(&logs))
}