use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct TftUserMatchListInput {
    puuid: String
}

pub async fn list_tft_matches_for_user_handler(data : web::Path<TftUserMatchListInput>, query: web::Query<api::PaginationParameters>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let matches = db::list_tft_match_summaries_for_puuid(
        &*app.pool,
        &data.puuid,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = matches.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(matches, &req, &query, expected_total == got_total)?)) 
}