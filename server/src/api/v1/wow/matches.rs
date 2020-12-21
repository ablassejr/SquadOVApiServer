use squadov_common::{
    SquadOvError,
    WoWEncounterStart,
    WoWChallengeStart,
    WoWEncounterEnd,
    WoWChallengeEnd,
    WoWCombatantInfo
};
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Executor, Postgres};
use serde::Deserialize;
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
    async fn create_wow_encounter_match<'a, T>(&self, ex: T, encounter: &WoWEncounterStart, combatants: &[WoWCombatantInfo], timestamp: &DateTime<Utc>, log_uuid: &Uuid) -> Result<Uuid, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        let uuid = Uuid::new_v4();
        Ok(uuid)
    }

    async fn finish_wow_encounter<'a, T>(&self, ex: T, match_uuid: &Uuid, timestamp: &DateTime<Utc>, encounter: &WoWEncounterEnd) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        Ok(())
    }

    async fn create_wow_challenge_match<'a, T>(&self, ex: T, challenge: &WoWChallengeStart, combatants: &[WoWCombatantInfo], timestamp: &DateTime<Utc>, log_uuid: &Uuid) -> Result<Uuid, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        let uuid = Uuid::new_v4();
        Ok(uuid)
    }

    async fn finish_wow_challenge<'a, T>(&self, ex: T, match_uuid: &Uuid, timestamp: &DateTime<Utc>, challenge: &WoWChallengeEnd) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        Ok(())
    }
}

pub async fn create_wow_encounter_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWEncounterStart>>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_encounter_match(&mut tx, &input_match.data, &input_match.combatants, &input_match.timestamp, &input_match.combat_log_uuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}

pub async fn create_wow_challenge_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWChallengeStart>>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_challenge_match(&mut tx, &input_match.data, &input_match.combatants, &input_match.timestamp, &input_match.combat_log_uuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(&uuid))
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