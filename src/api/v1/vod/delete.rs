use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};

pub async fn delete_vod_handler(app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    return Ok(HttpResponse::Ok().finish());
}