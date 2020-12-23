use squadov_common::SquadOvError;
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;

pub async fn get_kafka_credentials_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(&app.config.kafka.client_keypair))
}