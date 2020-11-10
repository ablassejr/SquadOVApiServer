use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use std::default::Default;
use uuid::Uuid;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct VodFindFromVideoUuid {
    video_uuid: Uuid,
}

impl api::ApiApplication {
    pub async fn get_vod_quality_options(&self, video_uuid: &Uuid) -> Result<Vec<squadov_common::VodMetadata>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            squadov_common::VodMetadata,
            "
            SELECT *
            FROM squadov.vod_metadata
            WHERE video_uuid = $1
            ",
            video_uuid
        ).fetch_all(&*self.pool).await?)
    }

    pub async fn get_vod(&self, video_uuid: &Uuid) -> Result<squadov_common::VodManifest, squadov_common::SquadOvError> {
        // We return our custom manifest format here instead of using M3U8 because we're not
        // going to be using a standard HLS player anyway and we're going to be using webm+opus
        // audio files which aren't standard HLS so it doesn't make sense to try and cram our
        // data into an M3U8 playlist. This way we have more flexibility in playing videos anyway so
        // all's good in the hood.
        let mut manifest = squadov_common::VodManifest{
            ..Default::default()
        };

        let quality_options = self.get_vod_quality_options(video_uuid).await?;
        for quality in &quality_options {
            let mut track = squadov_common::VodTrack{
                metadata: quality.clone(),
                segments: Vec::new(),
            };

            // Eventually we'll want to figure out how to do real segments and maintaining
            // compatability wit Electron but for now just a single file is all we have so just
            // pretend we just have a single segment.
            track.segments.push(squadov_common::VodSegment{
                uri: format!("/v1/vod/{video_uuid}/{quality}/video.mp4",
                    video_uuid=video_uuid,
                    quality=&quality.id,
                ),
                // Duration is a placeholder - not really needed but will be useful once we get
                // back to using semgnets.
                duration: 0.0,
                segment_start: 0.0,
            });

            manifest.video_tracks.push(track);
        }

        Ok(manifest)
    }
}

pub async fn get_vod_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let manifest = app.get_vod(&data.video_uuid).await?;
    Ok(HttpResponse::Ok().json(&manifest))
}

pub async fn get_vod_track_segment_handler(data : web::Path<squadov_common::VodSegmentId>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let redirect_uri = app.vod.get_segment_redirect_uri(&data).await?;
    // You may be tempted to make this into a TemporaryRedirect and point
    // a media player (e.g. VideoJS) to load from this directly. Don't do that
    // unless you can figure out how to also pass the user's session ID along
    // with that request since this is a protected endpoint.
    return Ok(HttpResponse::Ok().json(&redirect_uri))
}