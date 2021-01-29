use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodDeleteFromUuid {
    video_uuid: Uuid,
}

pub async fn delete_vod_handler(data : web::Path<VodDeleteFromUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let quality_options = app.get_vod_quality_options(&[data.video_uuid.clone()]).await?;
    let quality_arr = quality_options.get(&data.video_uuid);
    if quality_arr.is_some() {
        for quality in quality_arr.unwrap() {
            app.vod.delete_vod(&squadov_common::VodSegmentId{
                video_uuid: data.video_uuid.clone(),
                quality: quality.id.clone(),
                segment_name: String::from("video.mp4")
            }).await?;

            if quality.has_fastify {
                app.vod.delete_vod(&squadov_common::VodSegmentId{
                    video_uuid: data.video_uuid.clone(),
                    quality: quality.id.clone(),
                    segment_name: String::from("fastify.mp4")
                }).await?;
            }

            if quality.has_preview {
                app.vod.delete_vod(&squadov_common::VodSegmentId{
                    video_uuid: data.video_uuid.clone(),
                    quality: quality.id.clone(),
                    segment_name: String::from("preview.mp4")
                }).await?;
            }
        }
    }

    return Ok(HttpResponse::Ok().finish());
}