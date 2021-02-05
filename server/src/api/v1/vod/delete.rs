use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodDeleteFromUuid {
    video_uuid: Uuid,
}

pub async fn delete_vod_handler(data : web::Path<VodDeleteFromUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let quality_options = app.get_vod_quality_options(&[data.video_uuid.clone()]).await?;
    let quality_arr = quality_options.get(&data.video_uuid).ok_or(SquadOvError::NotFound)?;
    let vod = app.find_vod_associations(&[data.video_uuid.clone()]).await?.get(&data.video_uuid).ok_or(SquadOvError::NotFound)?.clone();
    let raw_extension = squadov_common::container_format_to_extension(&vod.raw_container_format);

    for quality in quality_arr {
        app.vod.delete_vod(&squadov_common::VodSegmentId{
            video_uuid: data.video_uuid.clone(),
            quality: quality.id.clone(),
            segment_name: format!("video.{}", &raw_extension),
        }).await?;

        if quality.has_fastify {
            app.vod.delete_vod(&squadov_common::VodSegmentId{
                video_uuid: data.video_uuid.clone(),
                quality: quality.id.clone(),
                segment_name: String::from("fastify.mp4"),
            }).await?;
        }

        if quality.has_preview {
            app.vod.delete_vod(&squadov_common::VodSegmentId{
                video_uuid: data.video_uuid.clone(),
                quality: quality.id.clone(),
                segment_name: String::from("preview.mp4"),
            }).await?;
        }
    }

    return Ok(HttpResponse::Ok().finish());
}