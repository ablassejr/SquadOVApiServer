use squadov_common::{
    SquadOvError,
    WoWEncounterStart,
    WoWChallengeStart,
    WoWArenaStart,
    WoWEncounterEnd,
    WoWChallengeEnd,
    WoWArenaEnd,
    WoWEncounter,
    WoWChallenge,
    WoWArena,
    WoWCombatantInfo,
    matches::MatchPlayerPair,
};
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use squadov_common::vod::VodAssociation;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Executor, Postgres};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct GenericMatchCreationRequest<T> {
    pub timestamp: DateTime<Utc>,
    pub combatants: Vec<WoWCombatantInfo>,
    pub data: T,
    #[serde(rename="combatLogUuid")]
    pub combat_log_uuid: Uuid
}

#[derive(Deserialize)]
pub struct GenericMatchFinishCreationRequest<T> {
    pub timestamp: DateTime<Utc>,
    pub data: T,
}

impl api::ApiApplication {
    async fn find_ongoing_wow_encounter_match<'a, T>(&self, ex: T, encounter: &WoWEncounterStart, combatants: &[WoWCombatantInfo]) -> Result<Option<Uuid>, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        Ok(
            sqlx::query_scalar(
                "
                SELECT match_uuid
                FROM squadov.wow_encounters
                WHERE encounter_id = $1
                    AND difficulty = $2
                    AND combatants_key = $3
                    AND finish_time IS NULL
                "
            )
                .bind(encounter.encounter_id)
                .bind(encounter.difficulty)
                .bind(squadov_common::generate_combatants_key(combatants))
                .fetch_optional(ex)
                .await?
        )
    }

    async fn add_wow_combatants_to_match<'a, T>(&self, ex: T, uuid: &Uuid, combatants: &[WoWCombatantInfo]) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.wow_match_combatants (
                match_uuid,
                combatant_guid
            )
            SELECT $1, *
            FROM UNNEST($2::VARCHAR[])
            ON CONFLICT DO NOTHING
            ",
            uuid,
            &combatants.iter().map(|x| { x.guid.clone() }).collect::<Vec<String>>()
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn link_wow_combat_log_to_match<'a, T>(&self, ex: T, match_uuid: &Uuid, log_uuid: &Uuid) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.wow_match_combat_log_association (
                match_uuid,
                combat_log_uuid
            )
            VALUES (
                $1,
                $2
            )
            ON CONFLICT DO NOTHING
            ",
            match_uuid,
            log_uuid
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn create_wow_encounter_match<'a, T>(&self, ex: T, uuid: &Uuid, encounter: &WoWEncounterStart, combatants: &[WoWCombatantInfo], timestamp: &DateTime<Utc>) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.wow_encounters (
                match_uuid,
                tm,
                combatants_key,
                encounter_id,
                encounter_name,
                difficulty,
                num_players,
                instance_id,
                success
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
                false
            )
            ",
            uuid,
            timestamp,
            squadov_common::generate_combatants_key(combatants),
            encounter.encounter_id,
            encounter.encounter_name,
            encounter.difficulty,
            encounter.num_players,
            encounter.instance_id
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn finish_wow_encounter<'a, T>(&self, ex: T, match_uuid: &Uuid, timestamp: &DateTime<Utc>, encounter: &WoWEncounterEnd) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            UPDATE squadov.wow_encounters
            SET finish_time = $2,
                success = $3
            WHERE match_uuid = $1
                AND finish_time IS NULL
            ",
            match_uuid,
            timestamp,
            encounter.success
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn find_ongoing_wow_challenge_match<'a, T>(&self, ex: T, challenge: &WoWChallengeStart, combatants: &[WoWCombatantInfo]) -> Result<Option<Uuid>, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        Ok(
            sqlx::query_scalar(
                "
                SELECT match_uuid
                FROM squadov.wow_challenges
                WHERE instance_id = $1
                    AND keystone_level = $2
                    AND combatants_key = $3
                    AND finish_time IS NULL
                "
            )
                .bind(challenge.instance_id)
                .bind(challenge.keystone_level)
                .bind(squadov_common::generate_combatants_key(combatants))
                .fetch_optional(ex)
                .await?
        )
    }

    async fn create_wow_challenge_match<'a, T>(&self, ex: T, uuid: &Uuid, challenge: &WoWChallengeStart, combatants: &[WoWCombatantInfo], timestamp: &DateTime<Utc>) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.wow_challenges (
                match_uuid,
                tm,
                combatants_key,
                challenge_name,
                instance_id,
                keystone_level,
                success,
                time_ms
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                false,
                0
            )
            ",
            uuid,
            timestamp,
            squadov_common::generate_combatants_key(combatants),
            challenge.challenge_name,
            challenge.instance_id,
            challenge.keystone_level,
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn finish_wow_challenge<'a, T>(&self, ex: T, match_uuid: &Uuid, timestamp: &DateTime<Utc>, challenge: &WoWChallengeEnd) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            UPDATE squadov.wow_challenges
            SET finish_time = $2,
                success = $3,
                time_ms = $4
            WHERE match_uuid = $1
                AND finish_time IS NULL
            ",
            match_uuid,
            timestamp,
            challenge.success,
            challenge.time_ms
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn find_ongoing_wow_arena_match<'a, T>(&self, ex: T, arena: &WoWArenaStart, combatants: &[WoWCombatantInfo]) -> Result<Option<Uuid>, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.wow_arenas
                WHERE instance_id = $1
                    AND arena_type = $2
                    AND combatants_key = $3
                    AND finish_time IS NULL
                ",
                arena.instance_id,
                arena.arena_type,
                squadov_common::generate_combatants_key(combatants),
            )
                .fetch_optional(ex)
                .await?
                .map(|x| {
                    x.match_uuid
                })
        )
    }

    async fn create_wow_arena_match<'a, T>(&self, ex: T, uuid: &Uuid, arena: &WoWArenaStart, combatants: &[WoWCombatantInfo], timestamp: &DateTime<Utc>) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.wow_arenas (
                match_uuid,
                tm,
                combatants_key,
                instance_id,
                arena_type
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5
            )
            ",
            uuid,
            timestamp,
            squadov_common::generate_combatants_key(combatants),
            arena.instance_id,
            &arena.arena_type,
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn finish_wow_arena<'a, T>(&self, ex: T, match_uuid: &Uuid, timestamp: &DateTime<Utc>, arena: &WoWArenaEnd) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            UPDATE squadov.wow_arenas
            SET finish_time = $2,
                winning_team_id = $3,
                match_duration_seconds = $4,
                new_ratings = $5
            WHERE match_uuid = $1
                AND finish_time IS NULL
            ",
            match_uuid,
            timestamp,
            arena.winning_team_id,
            arena.match_duration_seconds,
            &arena.new_ratings,
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn list_wow_encounters_for_character(&self, character_guid: &str, start: i64, end: i64) -> Result<Vec<WoWEncounter>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWEncounter,
                r#"
                SELECT DISTINCT we.*, wcl.build_version AS "build"
                FROM squadov.wow_encounters AS we
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE wmc.combatant_guid = $1
                ORDER BY tm DESC
                LIMIT $2 OFFSET $3
                "#,
                character_guid,
                end - start,
                start
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    pub async fn list_wow_encounter_for_uuids(&self, uuids: &[Uuid]) -> Result<Vec<WoWEncounter>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWEncounter,
                r#"
                SELECT DISTINCT we.*, wcl.build_version AS "build"
                FROM squadov.wow_encounters AS we
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE we.match_uuid = ANY($1)
                "#,
                uuids,
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    async fn list_wow_challenges_for_character(&self, character_guid: &str, start: i64, end: i64) -> Result<Vec<WoWChallenge>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWChallenge,
                r#"
                SELECT DISTINCT wc.*, wcl.build_version AS "build"
                FROM squadov.wow_challenges AS wc
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE wmc.combatant_guid = $1
                ORDER BY tm DESC
                LIMIT $2 OFFSET $3
                "#,
                character_guid,
                end - start,
                start
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    pub async fn list_wow_challenges_for_uuids(&self, uuids: &[Uuid]) -> Result<Vec<WoWChallenge>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWChallenge,
                r#"
                SELECT DISTINCT wc.*, wcl.build_version AS "build"
                FROM squadov.wow_challenges AS wc
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE wc.match_uuid = ANY($1)
                "#,
                uuids,
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    async fn list_wow_arenas_for_character(&self, character_guid: &str, start: i64, end: i64) -> Result<Vec<WoWArena>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWArena,
                r#"
                SELECT DISTINCT
                    wa.*,
                    (wcle.evt->>'team')::INTEGER = wa.winning_team_id AS "success!",
                    u.uuid AS "user_uuid!",
                    wcl.build_version AS "build"
                FROM squadov.wow_arenas AS wa
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wa.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wa.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                INNER JOIN squadov.wow_combat_log_events AS wcle
                    ON wcle.combat_log_uuid = wcl.uuid
                        AND wcle.evt @> '{"type": "CombatantInfo"}'
                        AND wcle.evt->>'guid' = wmc.combatant_guid
                        AND wcle.tm BETWEEN wa.tm AND wa.finish_time
                INNER JOIN squadov.wow_user_character_association AS wuca
                    ON wuca.guid = wmc.combatant_guid
                INNER JOIN squadov.users AS u
                    ON u.id = wuca.user_id
                WHERE wmc.combatant_guid = $1
                ORDER BY tm DESC
                LIMIT $2 OFFSET $3
                "#,
                character_guid,
                end - start,
                start
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    pub async fn list_wow_arenas_for_uuids(&self, uuids: &[MatchPlayerPair]) -> Result<Vec<WoWArena>, SquadOvError> {
        let match_uuids = uuids.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
        let player_uuids = uuids.iter().map(|x| { x.player_uuid.clone() }).collect::<Vec<Uuid>>();

        // We need to get the final list of match uuids to obtain since not all the match UUID/player UUID pairs
        // that we got are valid WoW arena runs. Return a list of (match uuid, player uuid, combatant guid).
        let final_identifiers = sqlx::query!(
            r#"
            SELECT
                inp.match_uuid AS "match_uuid!",
                inp.player_uuid AS "user_uuid!",
                wmc.combatant_guid AS "character_guid"
            FROM UNNEST($1::UUID[], $2::UUID[]) AS inp(match_uuid, player_uuid)
            INNER JOIN squadov.wow_match_combatants AS wmc
                ON wmc.match_uuid = inp.match_uuid
            INNER JOIN squadov.wow_user_character_association AS wuca
                ON wuca.guid = wmc.combatant_guid
            INNER JOIN squadov.users AS u
                ON u.id = wuca.user_id
                    AND u.uuid = inp.player_uuid
            "#,
            &match_uuids,
            &player_uuids,
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| {
                (x.match_uuid, x.user_uuid, x.character_guid)
            })
            .collect::<Vec<(Uuid, Uuid, String)>>();
        
        let match_uuids = final_identifiers.iter().map(|x| { x.0.clone() }).collect::<Vec<Uuid>>();
        let player_uuids = final_identifiers.iter().map(|x| { x.1.clone() }).collect::<Vec<Uuid>>();
        let character_guids = final_identifiers.iter().map(|x| { x.2.clone() }).collect::<Vec<String>>();

        Ok(
            sqlx::query_as!(
                WoWArena,
                r#"
                SELECT DISTINCT
                    wa.*,
                    (wcle.evt->>'team')::INTEGER = wa.winning_team_id AS "success!",
                    inp.player_uuid AS "user_uuid!",
                    wcl.build_version AS "build"
                FROM UNNEST($1::UUID[], $2::UUID[], $3::VARCHAR[]) AS inp(match_uuid, player_uuid, combatant_guid)
                INNER JOIN squadov.wow_arenas AS wa
                    ON wa.match_uuid = inp.match_uuid
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wa.match_uuid
                        AND wmc.combatant_guid = inp.combatant_guid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wa.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                INNER JOIN squadov.wow_combat_log_events AS wcle
                    ON wcle.combat_log_uuid = wcl.uuid
                        AND wcle.evt @> '{"type": "CombatantInfo"}'
                        AND wcle.evt->>'guid' = wmc.combatant_guid
                        AND wcle.tm BETWEEN wa.tm AND wa.finish_time
                "#,
                &match_uuids,
                &player_uuids,
                &character_guids,
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

    async fn find_wow_challenge(&self, match_uuid: &Uuid) -> Result<Option<WoWChallenge>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWChallenge,
                r#"
                SELECT DISTINCT wc.*, wcl.build_version AS "build"
                FROM squadov.wow_challenges AS wc
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wc.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE wc.match_uuid = $1
                "#,
                match_uuid
            )
                .fetch_optional(&*self.pool)
                .await?
        )
    }

    async fn find_wow_encounter(&self, match_uuid: &Uuid) -> Result<Option<WoWEncounter>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWEncounter,
                r#"
                SELECT DISTINCT we.*, wcl.build_version AS "build"
                FROM squadov.wow_encounters AS we
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = we.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                WHERE we.match_uuid = $1
                "#,
                match_uuid
            )
                .fetch_optional(&*self.pool)
                .await?
        )
    }

    async fn find_wow_arena(&self, match_uuid: &Uuid, user_id: i64) -> Result<Option<WoWArena>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                WoWArena,
                r#"
                SELECT DISTINCT
                    wa.*,
                    (wcle.evt->>'team')::INTEGER = wa.winning_team_id AS "success!",
                    u.uuid AS "user_uuid!",
                    wcl.build_version AS "build"
                FROM squadov.wow_arenas AS wa
                INNER JOIN squadov.wow_match_combatants AS wmc
                    ON wmc.match_uuid = wa.match_uuid
                INNER JOIN squadov.wow_match_combat_log_association AS cla
                    ON cla.match_uuid = wa.match_uuid
                INNER JOIN squadov.wow_combat_logs AS wcl
                    ON wcl.uuid = cla.combat_log_uuid
                INNER JOIN squadov.wow_combat_log_events AS wcle
                    ON wcle.combat_log_uuid = wcl.uuid
                        AND wcle.evt @> '{"type": "CombatantInfo"}'
                        AND wcle.evt->>'guid' = wmc.combatant_guid
                        AND wcle.tm BETWEEN wa.tm AND wa.finish_time
                INNER JOIN squadov.wow_user_character_association AS wuca
                    ON wuca.guid = wmc.combatant_guid
                INNER JOIN squadov.users AS u
                    ON u.id = wuca.user_id
                WHERE wa.match_uuid = $1 AND u.id = $2
                "#,
                match_uuid,
                user_id,
            )
                .fetch_optional(&*self.pool)
                .await?
        )
    }
}

pub async fn create_wow_encounter_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWEncounterStart>>) -> Result<HttpResponse, SquadOvError> {
    // Need to retry just in case we get a conflict.
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match app.find_ongoing_wow_encounter_match(&mut tx, &input_match.data, &input_match.combatants).await? {
            Some(uuid) => uuid,
            None => {
                let internal_match = app.create_new_match(&mut tx).await?;
                match app.create_wow_encounter_match(&mut tx, &internal_match.uuid, &input_match.data, &input_match.combatants, &input_match.timestamp).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        squadov_common::SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW encounter...retrying!");
                            tx.rollback().await?;
                            continue;
                        },
                        _ => return Err(err)
                    }
                }

                app.add_wow_combatants_to_match(&mut tx, &internal_match.uuid, &input_match.combatants).await?;
                internal_match.uuid
            }
        };

        // This needs to be outside the match block as this needs to be done regardless whether or not the incoming match is a duplicate!
        app.link_wow_combat_log_to_match(&mut tx, &match_uuid, &input_match.combat_log_uuid).await?;
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(squadov_common::SquadOvError::InternalError(String::from("WoW Encounter Match Retry Threshold")))
}

pub async fn create_wow_challenge_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWChallengeStart>>) -> Result<HttpResponse, SquadOvError> {
    // Need to retry just in case we get a conflict.
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match app.find_ongoing_wow_challenge_match(&mut tx, &input_match.data, &input_match.combatants).await? {
            Some(uuid) => uuid,
            None => {
                let internal_match = app.create_new_match(&mut tx).await?;
                match app.create_wow_challenge_match(&mut tx, &internal_match.uuid, &input_match.data, &input_match.combatants, &input_match.timestamp).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        squadov_common::SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW challenge...retrying!");
                            tx.rollback().await?;
                            continue;
                        },
                        _ => return Err(err)
                    }
                }
                app.add_wow_combatants_to_match(&mut tx, &internal_match.uuid, &input_match.combatants).await?;
                internal_match.uuid
            }
        };

        // This needs to be outside the match block as this needs to be done regardless whether or not the incoming match is a duplicate!
        app.link_wow_combat_log_to_match(&mut tx, &match_uuid, &input_match.combat_log_uuid).await?;
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(squadov_common::SquadOvError::InternalError(String::from("WoW Challenge Match Retry Threshold")))
}

pub async fn create_wow_arena_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWArenaStart>>) -> Result<HttpResponse, SquadOvError> {
    // Need to retry just in case we get a conflict.
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match app.find_ongoing_wow_arena_match(&mut tx, &input_match.data, &input_match.combatants).await? {
            Some(uuid) => uuid,
            None => {
                let internal_match = app.create_new_match(&mut tx).await?;
                match app.create_wow_arena_match(&mut tx, &internal_match.uuid, &input_match.data, &input_match.combatants, &input_match.timestamp).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        squadov_common::SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW arena...retrying!");
                            tx.rollback().await?;
                            continue;
                        },
                        _ => return Err(err)
                    }
                }
                app.add_wow_combatants_to_match(&mut tx, &internal_match.uuid, &input_match.combatants).await?;
                internal_match.uuid
            }
        };

        // This needs to be outside the match block as this needs to be done regardless whether or not the incoming match is a duplicate!
        app.link_wow_combat_log_to_match(&mut tx, &match_uuid, &input_match.combat_log_uuid).await?;
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(squadov_common::SquadOvError::InternalError(String::from("WoW Arena Match Retry Threshold")))
}

pub async fn finish_wow_encounter_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWEncounterEnd>>, path: web::Path<super::WoWMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.finish_wow_encounter(&mut tx, &path.match_uuid, &data.timestamp, &data.data).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn finish_wow_challenge_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWChallengeEnd>>, path: web::Path<super::WoWMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.finish_wow_challenge(&mut tx, &path.match_uuid, &data.timestamp, &data.data).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn finish_wow_arena_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWArenaEnd>>, path: web::Path<super::WoWMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.finish_wow_arena(&mut tx, &path.match_uuid, &data.timestamp, &data.data).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn list_wow_encounters_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let encounters = app.list_wow_encounters_for_character(
        &path.character_guid,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = encounters.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(encounters, &req, &query, expected_total == got_total)?))
}

pub async fn list_wow_challenges_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let challenges = app.list_wow_challenges_for_character(
        &path.character_guid,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = challenges.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(challenges, &req, &query, expected_total == got_total)?))
}

pub async fn list_wow_arenas_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let challenges = app.list_wow_arenas_for_character(
        &path.character_guid,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = challenges.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(challenges, &req, &query, expected_total == got_total)?))
}


pub async fn get_wow_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    #[derive(Serialize)]
    struct Response {
        encounter: Option<WoWEncounter>,
        challenge: Option<WoWChallenge>,
        arena: Option<WoWArena>,
    }

    Ok(HttpResponse::Ok().json(Response{
        encounter: app.find_wow_encounter(&path.match_uuid).await?,
        challenge: app.find_wow_challenge(&path.match_uuid).await?,
        arena: app.find_wow_arena(&path.match_uuid, path.user_id).await?,
    }))
}

#[derive(Serialize)]
struct WowUserAccessibleVodOutput {
    pub vods: Vec<VodAssociation>,
    #[serde(rename="userToId")]
    pub user_to_id: HashMap<Uuid, i64>
}

pub async fn list_wow_vods_for_squad_in_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    let vods = app.find_accessible_vods_in_match_for_user(&path.match_uuid, path.user_id).await?;

    // Note that for each VOD we also need to figure out the mapping from user uuid to puuid.
    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    let user_uuid_to_id = app.get_user_uuid_to_user_id_map(&user_uuids).await?;

    Ok(HttpResponse::Ok().json(WowUserAccessibleVodOutput{
        vods,
        user_to_id: user_uuid_to_id,
    }))
}