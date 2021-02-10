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
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RiotAccountVerifyData {
    game_name: String,
    tag_line: String,
    puuid: String,
}

#[derive(Deserialize)]
pub struct RiotSummonerPathInput {
    user_id: i64,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RiotSummonerVerifyData {
    summoner_name: String,
    puuid: String,
}

pub async fn get_riot_valorant_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotPuuidPathInput>) -> Result<HttpResponse, SquadOvError> {
    let account = db::get_user_riot_account(&*app.pool, path.user_id, &path.puuid).await?;
    Ok(HttpResponse::Ok().json(&account))
}

pub async fn verify_valorant_account_ownership_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotAccountPathInput>, data: web::Json<RiotAccountVerifyData>) -> Result<HttpResponse, SquadOvError> {
    if let Some(account) = db::get_user_riot_account_gamename_tagline(&*app.pool, path.user_id, &data.game_name, &data.tag_line).await? {
        // Associate the raw puuid with the encrypted puuid. We need to do this so that we can auto sync the account properly.
        db::associate_raw_puuid_with_puuid(&*app.pool, &account.puuid, &data.puuid).await?;
    } else if let Some(account) = db::get_user_riot_account_from_raw_puuid(&*app.pool, path.user_id, &data.puuid).await? {
        // In this case we have the account associated - the user updated their gamename/tagline so fire off a request to resync.
        app.valorant_itf.request_riot_account_from_puuid(&account.puuid).await?;
    } else {
        // No account!
        return Err(SquadOvError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}

pub async fn verify_lol_summoner_ownership_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotSummonerPathInput>, data: web::Json<RiotSummonerVerifyData>) -> Result<HttpResponse, SquadOvError> {
    if let Some(summoner) = db::get_user_riot_summoner_from_name(&*app.pool, path.user_id, &data.summoner_name).await? {
        // Associate the raw puuid with the encrypted puuid. We need to do this so that we can auto sync the account properly.
        db::associate_raw_puuid_with_puuid(&*app.pool, &summoner.puuid, &data.puuid).await?;
    } else if let Some(summoner) = db::get_user_riot_summoner_from_raw_puuid(&*app.pool, path.user_id, &data.puuid).await? {
        // In this case we have the account associated - the user updated their gamename/tagline so fire off a request to resync.
        app.lol_itf.request_lol_summoner_from_puuid(&summoner.puuid).await?;
    } else {
        // No account!
        return Err(SquadOvError::NotFound);
    }
    Ok(HttpResponse::NoContent().finish())
}

pub async fn verify_tft_summoner_ownership_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotSummonerPathInput>, data: web::Json<RiotSummonerVerifyData>) -> Result<HttpResponse, SquadOvError> {
    verify_lol_summoner_ownership_handler(app, path, data).await
}

pub async fn list_riot_lol_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_summoners_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

pub async fn list_riot_tft_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_summoners_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

pub async fn refresh_riot_account_from_puuid_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotPuuidPathInput>) -> Result<HttpResponse, SquadOvError> {
    app.valorant_itf.request_riot_account_from_puuid(&path.puuid).await?;
    app.lol_itf.request_lol_summoner_from_puuid(&path.puuid).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn delete_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotPuuidPathInput>) -> Result<HttpResponse, SquadOvError> {
    db::delete_riot_account(&*app.pool, &path.puuid).await?;
    Ok(HttpResponse::NoContent().finish())
}