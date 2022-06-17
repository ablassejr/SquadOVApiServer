use actix_web::{HttpResponse};
use squadov_common::SquadOvError;
use chrono::{Utc};

pub async fn get_server_time_handler() -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        Utc::now().timestamp_millis()
    ))
}