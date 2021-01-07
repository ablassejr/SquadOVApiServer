mod valorant;

pub use valorant::*;

use actix_web::{web, HttpResponse};
use crate::api;
use crate::api::v1::UserResourcePath;
use serde::Deserialize;
use std::sync::Arc;
use squadov_common::{SquadOvError, RiotAccount};
use squadov_common::riot::db;

// TODO: When we have access to Riot RSO this needs to handle that instead somehow.
pub async fn link_new_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>, data: web::Json<RiotAccount>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    db::store_riot_account(&mut tx, &data).await?;
    db::link_riot_account_to_user(&mut tx, path.user_id, &data.puuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn list_riot_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = db::list_riot_accounts_for_user(&*app.pool, path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

#[derive(Deserialize)]
pub struct RiotAccoutPathInput {
    user_id: i64,
    puuid: String
}

pub async fn get_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotAccoutPathInput>) -> Result<HttpResponse, SquadOvError> {
    let account = db::get_user_riot_account(&*app.pool, path.user_id, &path.puuid).await?;
    Ok(HttpResponse::Ok().json(&account))
}