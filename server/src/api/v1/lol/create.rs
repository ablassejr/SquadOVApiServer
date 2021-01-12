use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct LolCreateMatchInput {
    platform: String,
    #[serde(rename="matchId")]
    match_id: i64,
}

pub async fn create_lol_match_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<LolCreateMatchInput>) -> Result<HttpResponse, SquadOvError> {
    for _i in 0..2i32 {
        /*
        let mut tx = app.pool.begin().await?;
        let match_uuid = match db::create_or_get_match_uuid_for_valorant_match(&mut tx, &data.match_id).await {
            Ok(x) => x,
            Err(err) => match err {
                squadov_common::SquadOvError::Duplicate => {
                    log::warn!("Caught duplicate LoL match {:?}...retrying!", &data);
                    tx.rollback().await?;
                    continue;
                },
                _ => return Err(err)
            }
        };
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
        */
    }
    
    Err(SquadOvError::InternalError(String::from("Multiple failed attempts to create match uuid for LOL match exceeded retry threshold")))
}

pub async fn finish_lol_match_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn request_lol_match_backfill_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().finish())
}