use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ValorantBackfillInput {
    puuid: String
}

pub async fn request_valorant_match_backfill_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<ValorantBackfillInput>) -> Result<HttpResponse, SquadOvError> {
    app.valorant_itf.request_backfill_user_valorant_matches(&path.puuid).await?;
    Ok(HttpResponse::Ok().finish())
}