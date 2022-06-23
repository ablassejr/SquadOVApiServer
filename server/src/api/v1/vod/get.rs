use squadov_common::{
    SquadOvError,
    VodMetadata,
    VodManifest,
    VodDestination,
    VodSegmentId,
    vod::{
        db,
    }
};
use crate::{
    api::{
        self,
        auth::{SquadOvMachineId},
    },
};
use actix_web::{web, HttpResponse};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct VodFindFromVideoUuid {
    pub video_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct UploadPartQuery {
    // Should all be set or none be set.
    pub part: Option<i64>,
    pub session: Option<String>,
    pub bucket: Option<String>,
    pub accel: Option<i64>,
}

impl api::ApiApplication {
    pub async fn check_is_vod_fastify(&self, video_uuid: &Uuid) -> Result<bool, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT COALESCE(BOOL_OR(f.has_fastify), FALSE) AS "has_fastify!"
                FROM (
                    SELECT has_fastify
                    FROM squadov.vod_metadata
                    WHERE video_uuid = $1
                ) AS f
                "#,
                video_uuid,
            )
                .fetch_one(&*self.pool)
                .await?
                .has_fastify
        )
    }
    
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

    pub async fn get_vod_owner_user_id(&self, video_uuid: &Uuid) -> Result<i64, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT u.id
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE v.video_uuid = $1
                ",
                video_uuid
            )
                .fetch_one(&*self.pool)
                .await?
                .id
        )
    }

    pub async fn get_vod_match_uuid(&self, video_uuid: &Uuid) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.vods
                WHERE video_uuid = $1
                ",
                video_uuid
            )
                .fetch_one(&*self.pool)
                .await?
                .match_uuid
        )
    }

    pub async fn get_vod(&self, video_uuid: &[Uuid]) -> Result<HashMap<Uuid, VodManifest>, SquadOvError> {
        let quality_options = self.get_vod_quality_options(video_uuid).await?;
        let associations = self.find_vod_associations(video_uuid, "").await?;

        let mut ret: HashMap<Uuid, VodManifest> = HashMap::new();
        for (video_uuid, assoc) in associations {
            if !quality_options.contains_key(&video_uuid) {
                continue;
            }

            ret.insert(video_uuid.clone(), db::get_vod_manifest(&*self.pool, &assoc).await?);
        }
        Ok(ret)
    }
}

pub async fn get_vod_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let manifest = app.get_vod(&vec![data.video_uuid.clone()]).await?;
    let data = manifest.get(&data.video_uuid).ok_or(SquadOvError::NotFound)?;
    Ok(HttpResponse::Ok().json(data))
}

pub async fn get_vod_upload_path_handler(data : web::Path<VodFindFromVideoUuid>, query: web::Query<UploadPartQuery>, app : web::Data<Arc<api::ApiApplication>>, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let mut assocs = app.find_vod_associations(&[data.video_uuid.clone()], &machine_id.id).await?;
    let vod = assocs.remove(&data.video_uuid).ok_or(SquadOvError::NotFound)?;
    let accel = query.accel.unwrap_or(0) == 1;

    Ok(HttpResponse::Ok().json(&
        if let Some(session) = &query.session {
            if let Some(bucket) = &query.bucket {
                let part = query.part.unwrap_or(1);
                
                if part > 1 {
                    // If we have a session, bucket, and > 1 part, that means we already started the upload so it's a matter
                    // of figuring out the next URL to upload parts to.
                    let manager = app.get_vod_manager(&bucket).await?;
                    let extension = squadov_common::container_format_to_extension(&vod.raw_container_format);
                    VodDestination {
                        url: manager.get_segment_upload_uri(&VodSegmentId{
                            video_uuid: data.video_uuid.clone(),
                            quality: String::from("source"),
                            segment_name: format!("video.{}", extension),
                        }, session, part, accel).await?,
                        bucket: bucket.clone(),
                        session: session.clone(),
                        loc: manager.manager_type(),
                        purpose: manager.upload_purpose(),
                    }
                } else {
                    return Err(SquadOvError::BadRequest);
                }
            } else {
                return Err(SquadOvError::BadRequest);
            }
        } else {
            // If we get here, we're starting an upload for a VOD that is probably already tracked on our server.
            if let Some(expiration) = vod.expiration_time.as_ref() {
                if expiration < &Utc::now() {
                    return Err(SquadOvError::BadRequest);
                }
            }

            app.create_vod_destination(&data.video_uuid, &vod.raw_container_format, accel).await?
        }
    ))
}

pub async fn get_vod_association_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<Arc<api::ApiApplication>>, machine_id: Option<web::Header<SquadOvMachineId>>) -> Result<HttpResponse, SquadOvError> {
    let mut assocs = app.find_vod_associations(&[data.video_uuid.clone()], machine_id.map(|x| { x.id.clone() }).unwrap_or(String::new()).as_str()).await?;
    Ok(HttpResponse::Ok().json(assocs.remove(&data.video_uuid).ok_or(SquadOvError::NotFound)?))
}

#[derive(Deserialize)]
pub struct VodTrackQuery {
    pub md5: Option<i32>,
    pub expiration: Option<i32>,
}

pub async fn get_vod_fastify_status_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(app.check_is_vod_fastify(&data.video_uuid).await?))
}

pub async fn get_vod_track_segment_handler(data : web::Path<squadov_common::VodSegmentId>, app : web::Data<Arc<api::ApiApplication>>, query: web::Query<VodTrackQuery>) -> Result<HttpResponse, SquadOvError> {
    let metadata = db::get_vod_metadata(&*app.pool, &data.video_uuid, "source").await?;
    let manager = app.get_vod_manager(&metadata.bucket).await?;

    let (response_string, expiration) = if let Some(_md5) = query.md5 {
        (
            sqlx::query!(
                "
                SELECT md5
                FROM squadov.vods
                WHERE video_uuid = $1
                ",
                &data.video_uuid
            )
                .fetch_one(&*app.pool)
                .await?
                .md5
                .unwrap_or(String::new())
            ,
            None,
        )
    } else if !data.segment_name.starts_with("preview.") && db::check_if_vod_public(&*app.pool, &data.video_uuid).await? && manager.check_vod_segment_is_public(&data).await? {
        // If the VOD is public (shared), then we can return the public URL instead of the signed private one.
        (manager.get_public_segment_redirect_uri(&data).await?, None)
    } else {
        manager.get_segment_redirect_uri(&data, true).await?
    };


    Ok(
        if let Some(_exp) = query.expiration {
            #[derive(Serialize)]
            pub struct Response {
                url: String,
                expiration: Option<DateTime<Utc>>,
            }

            HttpResponse::Ok().json(&Response{
                url: response_string,
                expiration,
            })
        } else {
            // You may be tempted to make this into a TemporaryRedirect and point
            // a media player (e.g. VideoJS) to load from this directly. Don't do that
            // unless you can figure out how to also pass the user's session ID along
            // with that request since this is a protected endpoint.
            HttpResponse::Ok().json(&response_string)
        }
    )
}