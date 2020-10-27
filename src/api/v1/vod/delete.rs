use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodDeleteFromUuid {
    video_uuid: Uuid,
}

pub async fn delete_vod_handler(data : web::Path<VodDeleteFromUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, common::SquadOvError> {
    app.vod.delete_vod(&common::VodSegmentId{
        video_uuid: data.video_uuid.clone(),
        quality: String::from("source"),
        segment_name: String::from("video.mp4")
    }).await?;
    return Ok(HttpResponse::Ok().finish());
}