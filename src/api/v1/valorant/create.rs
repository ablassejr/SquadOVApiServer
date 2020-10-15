use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use uuid::Uuid;
use sqlx::{Transaction, Executor, Postgres};

use serde::{Serialize,Deserialize};

#[derive(Deserialize)]
pub struct InputValorantMatch {
    // Valorant unique ID
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "rawData")]
    pub raw_data: String
}

#[derive(Serialize)]
struct CreateValorantMatchResponse<'a> {
    #[serde(rename = "matchUuid")]
    match_uuid: &'a Uuid
}

impl api::ApiApplication {
    pub async fn check_if_valorant_match_exists(&self, match_id : &str) -> Result<bool, common::SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT EXISTS(
                SELECT *
                FROM squadov.valorant_matches
                WHERE match_id = $1
            )
            ",
        )  
            .bind(match_id)
            .fetch_one(&self.pool)
            .await?)
    }

    // TODO: When/if we get a production API key we need to have the user enter in the match UUID
    // and pull the data ourselves.
    pub async fn create_new_valorant_match(&self, tx : &mut Transaction<'_, Postgres>, uuid: &Uuid, raw_match : InputValorantMatch) -> Result<(), common::SquadOvError> {
        let full_match_data : super::FullValorantMatchData = serde_json::from_str(&raw_match.raw_data)?;     
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.valorant_matches (
                    match_id,
                    match_uuid,
                    game_mode,
                    map,
                    is_ranked,
                    provisioning_flow_id,
                    game_version,
                    server_start_time_utc,
                    raw_data
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
                ",
                &full_match_data.match_info.match_id,
                uuid,
                &full_match_data.match_info.game_mode,
                &full_match_data.match_info.map_id,
                full_match_data.match_info.is_ranked,
                &full_match_data.match_info.provisioning_flow_id,
                &full_match_data.match_info.game_version,
                full_match_data.match_info.server_start_time_utc,
                serde_json::from_str::<serde_json::Value>(&raw_match.raw_data)?
            )
        ).await?;
        return Ok(());
    }
}

pub async fn create_new_valorant_match_handler(data : web::Json<InputValorantMatch>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let raw_data = data.into_inner();
    // First check if this match exists.
    // If the match doesn't exist, then someone else reported it first - it's ok! The data should be the same.
    if app.check_if_valorant_match_exists(&raw_data.match_id).await? {
        return Ok(HttpResponse::Ok().finish())
    }

    let mut tx = app.pool.begin().await?;
    // Create a new match ID and then create the match.
    let internal_match = app.create_new_match(&mut tx).await?;
    app.create_new_valorant_match(&mut tx, &internal_match.uuid, raw_data).await?;
    tx.commit().await?;

    return Ok(HttpResponse::Ok().json(
        &CreateValorantMatchResponse{
            match_uuid: &internal_match.uuid
        }
    ))
}