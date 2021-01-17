use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;

pub async fn get_lol_match_handler(data : web::Path<super::LolMatchInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let lol_match = db::get_lol_match(&*app.pool, &data.match_uuid).await?;
    Ok(HttpResponse::Ok().json(&lol_match))
}