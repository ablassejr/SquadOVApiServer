mod valorant;

pub use valorant::*;

use actix_web::{web, HttpResponse};
use crate::api;
use crate::api::v1::UserResourcePath;
use serde::Deserialize;
use std::sync::Arc;
use squadov_common::{SquadOvError};
use squadov_common::riot::{
    db,
};

pub async fn list_riot_valorant_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_accounts_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

#[derive(Deserialize)]
pub struct RiotPuuidPathInput {
    user_id: i64,
    puuid: String
}

#[derive(Deserialize)]
pub struct RiotAccountPathInput {
    user_id: i64,
    game_name: String,
    tag_line: String
}

#[derive(Deserialize)]
pub struct RiotSummonerPathInput {
    user_id: i64,
    summoner_name: String
}

pub async fn get_riot_valorant_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotPuuidPathInput>) -> Result<HttpResponse, SquadOvError> {
    let account = db::get_user_riot_account(&*app.pool, path.user_id, &path.puuid).await?;
    Ok(HttpResponse::Ok().json(&account))
}

pub async fn get_riot_valorant_account_from_gamename_tagline_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotAccountPathInput>) -> Result<HttpResponse, SquadOvError> {
    let account = db::get_user_riot_account_gamename_tagline(&*app.pool, path.user_id, &path.game_name, &path.tag_line).await?;
    Ok(HttpResponse::Ok().json(&account))
}

pub async fn get_riot_lol_summoner_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotSummonerPathInput>) -> Result<HttpResponse, SquadOvError> {
    let summoner = db::get_user_riot_summoner_from_name(&*app.pool, path.user_id, &path.summoner_name).await?;
    Ok(HttpResponse::Ok().json(&summoner))
}

pub async fn get_riot_tft_summoner_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotSummonerPathInput>) -> Result<HttpResponse, SquadOvError> {
    let summoner = db::get_user_riot_summoner_from_name(&*app.pool, path.user_id, &path.summoner_name).await?;
    Ok(HttpResponse::Ok().json(&summoner))
}

pub async fn list_riot_lol_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_summoners_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

pub async fn list_riot_tft_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_summoners_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}