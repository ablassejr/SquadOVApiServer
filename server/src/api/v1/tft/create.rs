use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct TftCreateMatchInput {
    platform: String,
    region: String,
    #[serde(rename="matchId")]
    match_id: i64,
}

#[derive(Deserialize,Debug)]
pub struct TftBackfillInput {
    #[serde(rename="summonerName")]
    summoner_name: String,
    region: String,
}

#[derive(Deserialize,Debug)]
pub struct TftBackfillPath {
    user_id: i64
}

pub async fn create_tft_match_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<TftCreateMatchInput>) -> Result<HttpResponse, SquadOvError> {
    for _i in 0..2i32 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match db::create_or_get_match_uuid_for_tft_match(&mut tx, &data.platform, &data.region, data.match_id).await {
            Ok(x) => x,
            Err(err) => match err {
                squadov_common::SquadOvError::Duplicate => {
                    log::warn!("Caught duplicate TFT match {:?}...retrying!", &data);
                    tx.rollback().await?;
                    continue;
                },
                _ => return Err(err)
            }
        };
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    
    Err(SquadOvError::InternalError(String::from("Multiple failed attempts to create match uuid for TFT match exceeded retry threshold")))
}

pub async fn finish_tft_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::TftMatchInput>) -> Result<HttpResponse, SquadOvError> {
    let link = db::get_tft_match_link_from_uuid(&*app.pool, &path.match_uuid).await?;
    app.tft_itf.request_obtain_tft_match_info(&link.platform, &link.region, link.match_id, true).await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn request_tft_match_backfill_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<TftBackfillInput>, path: web::Path<TftBackfillPath>) -> Result<HttpResponse, SquadOvError> {
    app.tft_itf.request_backfill_user_tft_matches(&data.summoner_name, &data.region, path.user_id).await?;
    Ok(HttpResponse::Ok().finish())
}