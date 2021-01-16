use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;

pub async fn get_tft_match_handler(data : web::Path<super::TftMatchInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let tft_match = db::get_tft_match(&*app.pool, &data.match_uuid).await?;
    Ok(HttpResponse::Ok().json(&tft_match))
}