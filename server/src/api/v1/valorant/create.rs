use squadov_common::{
    SquadOvError,
    riot::db
};
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::{Deserialize};
use sqlx::{Transaction, Postgres};

#[derive(Deserialize)]
pub struct InputValorantMatch {
    // Valorant unique ID
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "playerData")]
    pub player_data: super::ValorantPlayerMatchMetadata
}

impl api::ApiApplication {
    async fn insert_valorant_player_round_data(&self, tx : &mut Transaction<'_, Postgres>, data: &[super::ValorantPlayerRoundMetadata]) -> Result<(), SquadOvError> {
        if data.is_empty() {
            return Ok(())
        }

        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_player_round_metadata (
                match_id,
                puuid,
                round,
                buy_time,
                round_time
            )
            VALUES
        "));

        for m in data {
            sql.push(format!("(
                '{match_id}',
                '{puuid}',
                {round},
                {buy_time},
                {round_time}
            )",
                match_id=&m.match_id,
                puuid=&m.puuid,
                round=m.round,
                buy_time=squadov_common::sql_format_option_some_time(m.buy_time.as_ref()),
                round_time=squadov_common::sql_format_option_some_time(m.round_time.as_ref())
            ));

            sql.push(String::from(","));
        }

        sql.truncate(sql.len() - 1);
        sqlx::query(&sql.join("")).execute(tx).await?;
        Ok(())
    }

    async fn insert_valorant_player_data(&self, tx : &mut Transaction<'_, Postgres>, player_data: &super::ValorantPlayerMatchMetadata) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.valorant_player_match_metadata (
                match_id,
                puuid,
                start_time,
                end_time
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            &player_data.match_id,
            &player_data.puuid,
            &player_data.start_time,
            &player_data.end_time
        )
            .execute(&mut *tx)
            .await?;

        self.insert_valorant_player_round_data(tx, &player_data.rounds).await?;
        Ok(())
    }
}

pub async fn create_new_valorant_match_handler(data : web::Json<InputValorantMatch>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    // Need to try multiple times to create a unique match uuid for the match in question.
    for _i in 0..2i32 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match db::create_or_get_match_uuid_for_valorant_match(&mut tx, &data.match_id).await {
            Ok(x) => x,
            Err(err) => match err {
                squadov_common::SquadOvError::Duplicate => {
                    // This indicates that the match UUID is INVALID because a match with the same
                    // match ID already exists. Retry!
                    log::warn!("Caught duplicate Valorant match {}...retrying!", &data.match_id);
                    tx.rollback().await?;
                    continue;
                },
                _ => return Err(err)
            }
        };
        app.insert_valorant_player_data(&mut tx, &data.player_data).await?;
        tx.commit().await?;

        app.valorant_itf.request_obtain_valorant_match_info(&data.match_id, true).await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    
    Err(SquadOvError::InternalError(String::from("Multiple failed attempts to create match uuid for Valorant match exceeded retry threshold")))
}