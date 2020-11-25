use squadov_common::SquadOvError;
use squadov_common::hearthstone::{HearthstonePlayer, HearthstonePlayerMedalInfo, FormatType, GameType};
use squadov_common::hearthstone::game_state::{HearthstoneGameBlock, HearthstoneGameSnapshot, HearthstoneGameSnapshotAuxData, HearthstoneGameAction, HearthstoneEntity, game_step::GameStep};
use squadov_common::hearthstone::game_packet::{HearthstoneMatchMetadata, HearthstoneGamePacket, HearthstoneSerializedGameLog};
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::auth::SquadOVSession;
use std::convert::TryFrom;
use std::collections::HashMap;

impl api::ApiApplication {
    pub async fn get_player_hero_entity_from_hearthstone_snapshot(&self, snapshot_uuid: &Uuid, user_id: i64) -> Result<HearthstoneEntity, SquadOvError> {
        let raw_entity = sqlx::query!(
            "
            SELECT
                hse.entity_id,
                hse.tags,
                hse.attributes
            FROM squadov.hearthstone_snapshots AS hs
            INNER JOIN squadov.hearthstone_match_players AS hmp
                ON hmp.match_uuid = hs.match_uuid
            INNER JOIN squadov.hearthstone_snapshots_player_map AS pm
                ON pm.player_id = hmp.player_match_id AND pm.snapshot_id = hs.snapshot_id
            INNER JOIN squadov.hearthstone_snapshots_entities AS hse
                ON hse.snapshot_id = hs.snapshot_id
            WHERE hse.snapshot_id = $1
                AND hmp.user_id = $2
                AND (hse.tags->>'CONTROLLER')::INTEGER = pm.player_id
                AND (hse.tags->>'CARDTYPE') = 'HERO'
            ORDER BY hse.entity_id ASC
            ",
            snapshot_uuid,
            user_id
        )
            .fetch_one(&*self.pool)
            .await?;
        
        Ok(HearthstoneEntity{
            entity_id: raw_entity.entity_id,
            tags: serde_json::from_value(raw_entity.tags)?,
            attributes: serde_json::from_value(raw_entity.attributes)?
        })
    }

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

    pub async fn get_hearthstone_snapshots_for_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<Uuid>, SquadOvError> {
        Ok(sqlx::query_scalar(
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
            .await?)
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
                    FROM squadov.hearthstone_snapshots
                    WHERE match_uuid = $1
                    ORDER BY last_action_id DESC
                    LIMIT 1
                )) - EXTRACT(EPOCH FROM (
                    SELECT tm
                    FROM squadov.hearthstone_snapshots
                    WHERE match_uuid = $1
                    ORDER BY last_action_id ASC
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
        
        let snapshot_ids = self.get_hearthstone_snapshots_for_match(match_uuid, user_id).await?;
        let latest_snapshot = match snapshot_ids.last() {
            Some(x) => Some(self.get_hearthstone_snapshot(x).await?),
            None => None
        };

        // Also retrieve the last snapshot of the game and just send it to the user "raw" and let them
        // parse it to retrieve some data about the final state of the game (e.g. this is what we're going
        // to use to display the final board state of the game).
        Ok(HearthstoneGamePacket{
            match_uuid: match_uuid.clone(),
            metadata: metadata,
            latest_snapshot: latest_snapshot
        })
    }

    pub async fn did_user_win_hearthstone_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<bool, SquadOvError> {
        let ret: Option<bool> = sqlx::query_scalar(
            "
            SELECT true
            FROM squadov.hearthstone_match_metadata AS hmm
            INNER JOIN squadov.hearethstone_match_players AS hmp
                ON hmp.match_uuid = hmm.match_uuid AND hmm.match_winner_player_id = hmp.player_match_id
            WHERE hmm.match_uuid = $1
                AND hmp.user_id = $2
            ",
        )
            .bind(match_uuid)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await?;

        Ok(ret.is_some())
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

        let action_blob_uuid: Uuid = sqlx::query_scalar(
            "
            SELECT actions_blob_uuid
            FROM squadov.hearthstone_match_action_blobs
            WHERE match_uuid = $1 AND user_id = $2
            ",
        )
            .bind(match_uuid)
            .bind(user_id)
            .fetch_one(&*self.pool)
            .await?;

        let raw_actions = self.blob.get_json_blob(&action_blob_uuid).await?;
        let actions : Vec<HearthstoneGameAction> = serde_json::from_value(raw_actions)?;
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
                    parent_block,
                    entity_id
                FROM squadov.hearthstone_blocks
                WHERE match_uuid = $1
                    AND user_id = $2
                ORDER BY end_action_index ASC
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