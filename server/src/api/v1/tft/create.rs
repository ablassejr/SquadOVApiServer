use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;

pub async fn create_tft_match_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn finish_tft_match_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().finish())
}

pub async fn request_tft_match_backfill_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().finish())
}