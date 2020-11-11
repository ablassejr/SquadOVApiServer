use squadov_common::SquadOvError;
use squadov_common::hearthstone::game_state::{HearthstoneGameBlock, HearthstoneGameSnapshot, HearthstoneGameAction, HearthstoneEntity, game_step::GameStep, EntityId};
use squadov_common::hearthstone::game_packet::{HearthstoneMatchMetadata, HearthstoneGamePacket, HearthstoneGameLogMetadata, HearthstoneSerializedSnapshot, HearthstoneSnapshotMetadata, HearthstoneSerializedGameLog};
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use uuid::Uuid;
use crate::api::auth::SquadOVSession;
use std::convert::TryFrom;

impl api::ApiApplication {

    pub async fn get_hearthstone_snapshot(&self, snapshot_uuid: &Uuid) -> Result<HearthstoneSerializedSnapshot, SquadOvError> {
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

        Ok(HearthstoneSerializedSnapshot{
            snapshot,
            metadata: HearthstoneSnapshotMetadata{
                current_turn: raw_metadata.current_turn,
                step: GameStep::try_from(raw_metadata.step)?,
                current_player_id: raw_metadata.current_player_id,
                last_action_id: raw_metadata.last_action_id
            }
        })
    }

    pub async fn get_hearthstone_match_for_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<HearthstoneGamePacket, SquadOvError> {
        // Give users some summary data about the match and then just dump the latest snapshot on them and let them figure out what
        // data they need from the snapshot on their own.
        let metadata = sqlx::query_as::<_,HearthstoneMatchMetadata>(
            "
            SELECT
                hmm.game_type,
                hmm.format_type,
                hmm.scenario_id,
                hm.match_time
            FROM squadov.hearthstone_matches AS hm
            INNER JOIN squadov.hearthstone_match_metadata AS hmm
                ON hmm.match_uuid = hm.match_uuid
            WHERE hm.match_uuid = $1
            ",
        )
            .bind(match_uuid)
            .fetch_one(&*self.pool)
            .await?;
        
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

        let mut snapshots: Vec<HearthstoneSerializedSnapshot> = Vec::new();
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