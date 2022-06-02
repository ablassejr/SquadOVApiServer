use squadov_common::{
    SquadOvError,
    blob,
    vod::db as vdb,
};
use squadov_common::hearthstone::game_state::{
    HearthstoneEntity,
    game_step::GameStep
};
use squadov_common::hearthstone::db as hdb;
use squadov_common::vod::VodAssociation;
use crate::api;
use crate::api::auth::{SquadOvMachineId, SquadOVSession};
use crate::api::v1::GenericMatchPathInput;
use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use std::sync::Arc;
use uuid::Uuid;
use std::convert::TryFrom;
use std::collections::HashMap;
use std::iter::FromIterator;
use serde::Serialize;
use squadov_common::proto::hearthstone::{
    HearthstoneSerializedGameSnapshot,
    HearthstoneSerializedEntity,
    HearthstoneSerializedGameSnapshotAuxData,
    HearthstoneSerializedGameBlock,
    HearthstoneSerializedGameLog
};
use prost::Message;

impl api::ApiApplication {
    pub async fn get_player_hero_entity_from_hearthstone_snapshot(&self, snapshot_uuid: &Uuid, user_id: i64) -> Result<HearthstoneEntity, SquadOvError> {
        let raw_entity = sqlx::query!(
            "
            SELECT
                hse.entity_id,
                hse.tags,
                hse.attributes
            FROM squadov.hearthstone_snapshots AS hs
            INNER JOIN squadov.hearthstone_match_view AS hmv
                ON hmv.match_uuid = hs.match_uuid AND hmv.user_id = hs.user_id
            INNER JOIN squadov.hearthstone_match_players AS hmp
                ON hmp.view_uuid = hmv.view_uuid
            INNER JOIN squadov.hearthstone_snapshots_player_map AS pm
                ON pm.player_id = hmp.player_match_id AND pm.snapshot_id = hs.snapshot_id
            INNER JOIN squadov.hearthstone_snapshots_entities AS hse
                ON hse.snapshot_id = hs.snapshot_id
            INNER JOIN squadov.hearthstone_match_metadata AS hmm
                ON hmm.view_uuid = hmv.view_uuid
            WHERE hse.snapshot_id = $1
                AND hmp.user_id = $2
                AND (hse.tags->>'CONTROLLER')::INTEGER = pm.player_id
                AND (hse.tags->>'CARDTYPE') = 'HERO'
                AND ((hse.tags->>'ZONE') = 'PLAY' OR (hse.tags->>'ZONE') = 'GRAVEYARD')
                AND ((hmm.game_type = 23 AND (hse.tags->>'PLAYER_ID') IS NOT NULL) OR hmm.game_type != 23) 
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

    pub async fn get_bulk_hearthstone_serialized_snapshots(&self, snapshot_uuids: &[Uuid]) -> Result<Vec<HearthstoneSerializedGameSnapshot>, SquadOvError> {
        let mut all_snapshots: HashMap<Uuid, HearthstoneSerializedGameSnapshot> = HashMap::from_iter(sqlx::query!(
            "
            SELECT
                tm,
                game_entity_id,
                current_turn,
                step,
                current_player_id,
                last_action_id,
                snapshot_id
            FROM squadov.hearthstone_snapshots
            WHERE snapshot_id = any($1)
            ",
            snapshot_uuids
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| {
                (x.snapshot_id.clone(), HearthstoneSerializedGameSnapshot{
                    uuid: x.snapshot_id.to_string(),
                    tm: match x.tm {
                        Some(t) => t.timestamp(),
                        None => 0,
                    },
                    game_entity_id: x.game_entity_id,
                    aux_data: Some(HearthstoneSerializedGameSnapshotAuxData{
                        current_turn: x.current_turn,
                        step: GameStep::try_from(x.step).unwrap_or(GameStep::Invalid) as i32,
                        current_player_id: x.current_player_id,
                        last_action_index: x.last_action_id as u64
                    }),
                    player_name_to_player_id: HashMap::new(),
                    player_id_to_entity_id: HashMap::new(),
                    entities: HashMap::new(),
                })
            }));

        let all_players = sqlx::query!(
            "
            SELECT 
                player_name,
                player_id,
                entity_id,
                snapshot_id
            FROM squadov.hearthstone_snapshots_player_map
            WHERE snapshot_id = any($1)
            ",
            snapshot_uuids
        )
            .fetch_all(&*self.pool)
            .await?;

        for sp in all_players {
            let snapshot = all_snapshots.get_mut(&sp.snapshot_id);
            if snapshot.is_none() {
                continue
            }
            let snapshot = snapshot.unwrap();
            snapshot.player_name_to_player_id.insert(sp.player_name, sp.player_id);
            snapshot.player_id_to_entity_id.insert(sp.player_id, sp.entity_id);
        }

        let snapshot_entities = sqlx::query!(
            r#"
            SELECT
                entity_id,
                tags::VARCHAR AS "tags!",
                attributes::VARCHAR AS "attributes!",
                snapshot_id
            FROM squadov.hearthstone_snapshots_entities
            WHERE snapshot_id = any($1)
            "#,
            snapshot_uuids
        )
            .fetch_all(&*self.pool)
            .await?;

        for se in snapshot_entities {
            let snapshot = all_snapshots.get_mut(&se.snapshot_id);
            if snapshot.is_none() {
                continue
            }
            let snapshot = snapshot.unwrap();
            let entity = HearthstoneSerializedEntity{
                entity_id: se.entity_id,
                tags: se.tags,
                attributes: se.attributes
            };

            snapshot.entities.insert(se.entity_id, entity);
        }

        let mut ret_snapshots = all_snapshots
            .into_iter()
            .map(|(_k, v)|{ v })
            .collect::<Vec<HearthstoneSerializedGameSnapshot>>();

        ret_snapshots.sort_by(|a, b| {
            a.aux_data.as_ref().unwrap().last_action_index.cmp(&b.aux_data.as_ref().unwrap().last_action_index)
        });

        Ok(ret_snapshots)
    }

    pub async fn did_user_win_hearthstone_match(&self, match_uuid: &Uuid, user_id: i64) -> Result<bool, SquadOvError> {
        let ret: Option<bool> = sqlx::query_scalar(
            "
            SELECT true
            FROM squadov.hearthstone_match_metadata AS hmm
            INNER JOIN squadov.hearthstone_match_players AS hmp
                ON hmp.view_uuid = hmm.view_uuid AND hmm.match_winner_player_id = hmp.player_match_id
            INNER JOIN squadov.hearthstone_match_view AS hmv
                ON hmv.view_uuid = hmp.view_uuid
            WHERE hmv.match_uuid = $1
                AND hmv.user_id = $2
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
        let snapshots: Vec<HearthstoneSerializedGameSnapshot> = self.get_bulk_hearthstone_serialized_snapshots(&snapshot_ids).await?;

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

        let bucket = blob::get_blob_bucket(&*self.pool, &action_blob_uuid).await?;
        let manager = self.get_blob_manager(&bucket).await?;

        let raw_actions = manager.get_blob(&action_blob_uuid, true).await?;
        let blocks: Vec<HearthstoneSerializedGameBlock> = sqlx::query!(
            r#"
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
            "#,
            match_uuid,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| {
                HearthstoneSerializedGameBlock{
                    block_id: x.block_id.to_string(),
                    start_action_index: x.start_action_index,
                    end_action_index: x.end_action_index,
                    block_type: x.block_type,
                    parent_block: match x.parent_block {
                        Some(u) => u.to_string(),
                        None => String::new(),
                    },
                    entity_id: x.entity_id,
                }
            })
            .collect();
        Ok(HearthstoneSerializedGameLog{
            snapshots,
            actions: String::from(std::str::from_utf8(&raw_actions)?),
            blocks,
        })
    }

    pub async fn get_hearthstone_match_hero_cards_for_user_uuids(&self, match_uuid: &Uuid, view_user_ids: &[i64]) -> Result<HashMap<i64, String>, SquadOvError> {
        // It's tempting to write a single SQL query to grab this info but I don't think it's worth the complexity versus just using what already exists.
        let mut user_id_to_hero_card : HashMap<i64, String> = HashMap::new();
        for uid in view_user_ids {
            let snapshot_ids = hdb::get_hearthstone_snapshots_for_match(&*self.pool, match_uuid, *uid).await?;
            let latest_snapshot = match snapshot_ids.last() {
                Some(x) => x,
                None => return Err(SquadOvError::NotFound)
            };

            let entity = self.get_player_hero_entity_from_hearthstone_snapshot(&latest_snapshot, *uid).await?;
            user_id_to_hero_card.insert(*uid, entity.card_id().unwrap_or(String::from("<UNKNOWN>")));
        }

        Ok(user_id_to_hero_card)
    }
}

pub async fn get_hearthstone_match_handler(path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let packet = hdb::get_hearthstone_game_packet(&*app.pool, &path.match_uuid, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&packet))
}

pub async fn get_hearthstone_match_logs_handler(path : web::Path<super::HearthstoneMatchGetInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    // The bottleneck in this function is the get_hearthstone_match_logs_for_user function.
    // Namely the most expensive part is fully the thousands of entities in each snapshot.
    let logs = app.get_hearthstone_match_logs_for_user(&path.match_uuid, path.user_id).await?;

    let mut buf: Vec<u8> = vec![];
    logs.encode(&mut buf)?;

    Ok(HttpResponse::Ok().body(buf))
}

#[derive(Serialize)]
struct HearthstoneUserAccessibleVodOutput {
    pub vods: Vec<VodAssociation>,
    #[serde(rename="userToHero")]
    pub user_to_hero: HashMap<i64, String>,
    #[serde(rename="userToId")]
    pub user_to_id: HashMap<Uuid, i64>,
}


pub async fn get_hearthstone_match_user_accessible_vod_handler(data: web::Path<GenericMatchPathInput>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };
    let vods = vdb::find_accessible_vods_in_match_for_user(&*app.pool, &data.match_uuid, session.user.id, &machine_id.id).await?;

    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    let user_uuid_to_id = app.get_user_uuid_to_user_id_map(&user_uuids).await?;
    let user_ids: Vec<i64> = user_uuids.iter().map(|x| { user_uuid_to_id.get(x).cloned().unwrap_or(-1) }).filter(|x| { *x != -1 }).collect();

    // We need to tell the user the hero card for each VOD so the UI knows what to display (note that we can identify VODs uniquely by user uuid at this point).
    // Additionally, need to match the user UUID to user ID since the UI generally works with user IDs instead of user UUIDs.
    Ok(HttpResponse::Ok().json(HearthstoneUserAccessibleVodOutput{
        vods,
        user_to_hero: app.get_hearthstone_match_hero_cards_for_user_uuids(&data.match_uuid, &user_ids).await?,
        user_to_id: user_uuid_to_id,
    }))
}