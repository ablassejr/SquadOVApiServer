use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::Deserialize;
use uuid::Uuid;
use chrono::{DateTime,Utc};

#[derive(Deserialize,Debug)]
pub struct LolCreateMatchInput {
    platform: String,
    #[serde(rename="matchId")]
    match_id: i64,
    #[serde(rename="gameStartTime")]
    game_start_time: DateTime<Utc>,
}

#[derive(Deserialize,Debug)]
pub struct LolMatchInput {
    match_uuid: Uuid
}

#[derive(Deserialize,Debug)]
pub struct LolBackfillInput {
    #[serde(rename="summonerName")]
    summoner_name: String,
    platform: String,
}

#[derive(Deserialize,Debug)]
pub struct LolBackfillPath {
    user_id: i64
}

pub async fn create_lol_match_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<LolCreateMatchInput>) -> Result<HttpResponse, SquadOvError> {
    for _i in 0..2i32 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match db::create_or_get_match_uuid_for_lol_match(&mut tx, &data.platform, data.match_id, Some(data.game_start_time.clone())).await {
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
    }
    
    Err(SquadOvError::InternalError(String::from("Multiple failed attempts to create match uuid for LOL match exceeded retry threshold")))
}

pub async fn finish_lol_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<LolMatchInput>) -> Result<HttpResponse, SquadOvError> {
    let link = db::get_lol_match_link_from_uuid(&*app.pool, &path.match_uuid).await?;
    app.lol_itf.request_obtain_lol_match_info(&link.platform, link.match_id, true).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn request_lol_match_backfill_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<LolBackfillInput>, path: web::Path<LolBackfillPath>) -> Result<HttpResponse, SquadOvError> {
    app.lol_itf.request_backfill_user_lol_matches(&data.summoner_name, &data.platform, path.user_id).await?;
    Ok(HttpResponse::Ok().finish())
}