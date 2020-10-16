use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};

pub async fn get_aimlab_task_data_handler(data : web::Path<super::AimlabTaskGetInput>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    return Ok(HttpResponse::Ok().finish())
}