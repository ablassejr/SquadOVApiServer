use squadov_common::SquadOvError;
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
struct KafkaInfo {
    servers: String,
    key: String,
    secret: String,
    wow_topic: String,
}

pub async fn get_kafka_info_handler(app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(KafkaInfo{
        servers: app.config.kafka.bootstrap_servers.clone(),
        key: app.config.kafka.client_keypair.key.clone(),
        secret: app.config.kafka.client_keypair.secret.clone(),
        wow_topic: app.config.kafka.wow_combat_log_topic.clone(),
    }))
}