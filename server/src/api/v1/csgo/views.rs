use actix_web::{web, HttpResponse};
use crate::api;
use squadov_common::{
    SquadOvError,
    csgo::gsi::CsgoGsiMatchState,
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct CsgoCreateViewPath {
    user_id: i64
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoCreateViewData {
    server: String,
    start_time: DateTime<Utc>,
    map: String,
    mode: String,
}

#[derive(Deserialize,Debug)]
pub struct CsgoViewPath {
    user_id: i64,
    view_uuid: Uuid,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoViewData {
    stop_time: DateTime<Utc>,
    match_state: CsgoGsiMatchState,
    demo: Option<String>,
}

pub async fn create_csgo_view_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CsgoCreateViewPath>, data: web::Json<CsgoCreateViewData>) -> Result<HttpResponse, SquadOvError> {
    Err(SquadOvError::NotFound)
}

pub async fn finish_csgo_view_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CsgoViewPath>, data: web::Json<CsgoViewData>) -> Result<HttpResponse, SquadOvError> {
    Err(SquadOvError::NotFound)
}