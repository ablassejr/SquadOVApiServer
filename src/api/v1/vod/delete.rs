use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;

pub async fn delete_vod_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, common::SquadOvError> {
    return Ok(HttpResponse::Ok().finish());
}