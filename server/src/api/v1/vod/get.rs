use squadov_common::{
    SquadOvError,
    VodMetadata,
    VodSegment,
    VodManifest,
    VodTrack,
};
use crate::api;
use actix_web::{web, HttpResponse};
use serde::{Deserialize};
use std::default::Default;
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct VodFindFromVideoUuid {
    video_uuid: Uuid,
}

impl api::ApiApplication {
    pub async fn get_vod_quality_options(&self, video_uuid: &[Uuid]) -> Result<HashMap<Uuid, Vec<VodMetadata>>, SquadOvError> {
        let metadata = sqlx::query_as!(
            VodMetadata,
            "
            SELECT *
            FROM squadov.vod_metadata
            WHERE video_uuid = ANY($1)
            ",
            video_uuid
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret = HashMap::new();
        metadata.into_iter().for_each(|x| {
            if !ret.contains_key(&x.video_uuid) {
                ret.insert(x.video_uuid.clone(), Vec::new());
            }
            let arr = ret.get_mut(&x.video_uuid).unwrap();
            arr.push(x);
        });
        Ok(ret)
    }

    pub async fn get_vod(&self, video_uuid: &[Uuid]) -> Result<HashMap<Uuid, VodManifest>, SquadOvError> {
        let quality_options = self.get_vod_quality_options(video_uuid).await?;
        let associations = self.find_vod_associations(video_uuid).await?;

        Ok(
            video_uuid.iter().map(|x| {
                // We return our custom manifest format here instead of using M3U8 because we're not
                // going to be using a standard HLS player anyway and we're going to be using webm+opus
                // audio files which aren't standard HLS so it doesn't make sense to try and cram our
                // data into an M3U8 playlist. This way we have more flexibility in playing videos anyway so
                // all's good in the hood.
                let mut manifest = VodManifest{
                    ..Default::default()
                };

                let options = quality_options.get(x).ok_or(SquadOvError::NotFound)?;
                let assoc = associations.get(x).ok_or(SquadOvError::NotFound)?;

                for quality in options {
                    let mut track = VodTrack{
                        metadata: quality.clone(),
                        segments: Vec::new(),
                        preview: None,
                    };

                    let container_format = String::from(if quality.has_fastify {
                        "mp4"
                    } else {
                        &assoc.raw_container_format
                    });

                    // Eventually we'll want to figure out how to do real segments and maintaining
                    // compatability wit Electron but for now just a single file is all we have so just
                    // pretend we just have a single segment.
                    track.segments.push(VodSegment{
                        uri: format!("/v1/vod/{video_uuid}/{quality}/{segment}.{extension}",
                            video_uuid=x,
                            quality=&quality.id,
                            segment=if quality.has_fastify {
                                "fastify"
                            } else {
                                "video"
                            },
                            extension=&squadov_common::container_format_to_extension(&container_format),
                        ),
                        // Duration is a placeholder - not really needed but will be useful once we get
                        // back to using semgnets.
                        duration: 0.0,
                        segment_start: 0.0,
                        mime_type: squadov_common::container_format_to_mime_type(&container_format),
                    });

                    if quality.has_preview {
                        track.preview = Some(format!(
                            "/v1/vod/{video_uuid}/{quality}/preview.mp4",
                            video_uuid=x,
                            quality=&quality.id,
                        ));
                    }

                    manifest.video_tracks.push(track);
                }

                Ok((x.clone(), manifest))
            }).collect::<Result<HashMap<Uuid, VodManifest>, SquadOvError>>()?
        )
    }
}

pub async fn get_vod_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let manifest = app.get_vod(&vec![data.video_uuid.clone()]).await?;
    let data = manifest.get(&data.video_uuid).ok_or(SquadOvError::NotFound)?;
    Ok(HttpResponse::Ok().json(data))
}

pub async fn get_vod_track_segment_handler(data : web::Path<squadov_common::VodSegmentId>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let redirect_uri = app.vod.get_segment_redirect_uri(&data).await?;
    // You may be tempted to make this into a TemporaryRedirect and point
    // a media player (e.g. VideoJS) to load from this directly. Don't do that
    // unless you can figure out how to also pass the user's session ID along
    // with that request since this is a protected endpoint.
    return Ok(HttpResponse::Ok().json(&redirect_uri))
}