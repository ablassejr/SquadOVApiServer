use squadov_common::{
    SquadOvError,
    WoWEncounterStart,
    WoWChallengeStart,
    WoWEncounterEnd,
    WoWChallengeEnd,
    WoWEncounter,
    WoWChallenge,
    WoWCombatantInfo,
};
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Executor, Postgres};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

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
                app.link_wow_combat_log_to_match(&mut tx, &internal_match.uuid, &input_match.combat_log_uuid).await?;
                internal_match.uuid
            }
        };
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
                app.link_wow_combat_log_to_match(&mut tx, &internal_match.uuid, &input_match.combat_log_uuid).await?;
                internal_match.uuid
            }
        };
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(squadov_common::SquadOvError::InternalError(String::from("WoW Challenge Match Retry Threshold")))
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

pub async fn get_wow_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    #[derive(Serialize)]
    struct Response {
        encounter: Option<WoWEncounter>,
        challenge: Option<WoWChallenge>,
    }

    Ok(HttpResponse::Ok().json(Response{
        encounter: app.find_wow_encounter(&path.match_uuid).await?,
        challenge: app.find_wow_challenge(&path.match_uuid).await?,
    }))
}