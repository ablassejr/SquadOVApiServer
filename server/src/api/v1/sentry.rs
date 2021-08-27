use actix_web::{web, HttpResponse};
use squadov_common::SquadOvError;
use crate::api::ApiApplication;
use std::sync::Arc;

pub async fn get_desktop_sentry_info_handler(app : web::Data<Arc<ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(app.config.sentry.client_service_dsn.clone()))
}

pub async fn get_web_sentry_info_handler(app : web::Data<Arc<ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(app.config.sentry.web_dsn.clone()))
}