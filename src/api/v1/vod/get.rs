use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use std::default::Default;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodFindFromVideoUuid {
    video_uuid: Uuid,
}

impl api::ApiApplication {
    pub async fn get_vod_quality_options(&self, video_uuid: &Uuid) -> Result<Vec<common::VodMetadata>, common::SquadOvError> {
        Ok(sqlx::query_as!(
            common::VodMetadata,
            "
            SELECT *
            FROM squadov.vod_metadata
            WHERE video_uuid = $1
            ",
            video_uuid
        ).fetch_all(&*self.pool).await?)
    }

    pub async fn get_vod(&self, video_uuid: &Uuid) -> Result<common::VodManifest, common::SquadOvError> {
        // We return our custom manifest format here instead of using M3U8 because we're not
        // going to be using a standard HLS player anyway and we're going to be using webm+opus
        // audio files which aren't standard HLS so it doesn't make sense to try and cram our
        // data into an M3U8 playlist. This way we have more flexibility in playing videos anyway so
        // all's good in the hood.
        let mut manifest = common::VodManifest{
            ..Default::default()
        };

        let quality_options = self.get_vod_quality_options(video_uuid).await?;
        for quality in &quality_options {
            let mut track = common::VodTrack{
                metadata: quality.clone(),
                segments: Vec::new(),
            };

            // Eventually we'll want to figure out how to do real segments and maintaining
            // compatability wit Electron but for now just a single file is all we have so just
            // pretend we just have a single segment.
            track.segments.push(common::VodSegment{
                uri: format!("/v1/vod/{video_uuid}/{quality}/{segment}",
                    video_uuid=video_uuid,
                    quality=&quality.id,
                    segment=&quality.fname,
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

pub async fn get_vod_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let manifest = app.get_vod(&data.video_uuid).await?;
    Ok(HttpResponse::Ok().json(&manifest))
}

pub async fn get_vod_track_segment_handler(data : web::Path<common::VodSegmentId>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let redirect_uri = app.vod.get_segment_redirect_uri(&data)?;
    return Ok(HttpResponse::Ok().json(&redirect_uri))
}