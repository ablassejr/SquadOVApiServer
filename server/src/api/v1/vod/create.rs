use async_trait::async_trait;
use squadov_common;
use squadov_common::vod;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use sqlx::{Executor};
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;
use sqlx::{Transaction, Postgres};
use tempfile::NamedTempFile;

#[derive(Deserialize)]
pub struct VodCreateDestinationUriInput {
    #[serde(rename="videoUuid")]
    video_uuid: Uuid
}

#[derive(Deserialize)]
pub struct VodAssociatePathInput {
    video_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct VodAssociateBodyInput {
    association: super::VodAssociation,
    metadata: squadov_common::VodMetadata,
    #[serde(rename="sessionUri")]
    session_uri: Option<String>,
}

pub struct VodFastifyJob {
    pub video_uuid: Uuid,
    pub app: Arc<api::ApiApplication>,
    pub session_uri: Option<String>,
}

pub struct VodFastifyWorker {

}

#[async_trait]
impl squadov_common::JobWorker<VodFastifyJob> for VodFastifyWorker {
    fn new() -> Self {
        Self {}
    }

    async fn work(&self, data: &VodFastifyJob) -> Result<(), squadov_common::SquadOvError> {
        log::info!("Start Fastifying {:?}", &data.video_uuid);

        // Note that we can only proceed with "fastifying" the VOD if the entire VOD has been uploaded.
        // We can query GCS's XML API to determine this. If the GCS Session URI is not provided then
        // we assume that the file has already been fully uploaded. If the file hasn't been fully uploaded
        // then we want to defer taking care of this task until later.
        if data.session_uri.is_some() {
            let session_uri = data.session_uri.as_ref().unwrap();
            if !data.app.vod.is_vod_session_finished(&session_uri).await? {
                log::info!("Defer Fastifying {:?}", &data.video_uuid);
                return Err(squadov_common::SquadOvError::Defer);
            }
        }

        data.app.fastify_vod(&data.video_uuid).await?;
        log::info!("Finish Fastifying {:?}", &data.video_uuid);
        Ok(())
    }
}

impl api::ApiApplication {
    pub async fn associate_vod(&self, tx : &mut Transaction<'_, Postgres>, assoc : &super::VodAssociation) -> Result<(), squadov_common::SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                UPDATE squadov.vods
                SET match_uuid = $1,
                    user_uuid = $2,
                    start_time = $3,
                    end_time = $4
                WHERE video_uuid = $5
                ",
                assoc.match_uuid,
                assoc.user_uuid,
                assoc.start_time,
                assoc.end_time,
                assoc.video_uuid
            )
        ).await?;
        Ok(())
    }

    pub async fn reserve_vod_uuid(&self, vod_uuid: &Uuid, user_id: i64) -> Result<(), squadov_common::SquadOvError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "
            INSERT INTO squadov.vods (video_uuid, user_uuid)
            SELECT $1, u.uuid
            FROM squadov.users AS u
            WHERE u.id = $2
            ",
            vod_uuid,
            user_id,
        )
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn bulk_add_video_metadata(&self, tx : &mut Transaction<'_, Postgres>, vod_uuid: &Uuid, data: &[squadov_common::VodMetadata]) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.vod_metadata (
                video_uuid,
                res_x,
                res_y,
                min_bitrate,
                avg_bitrate,
                max_bitrate,
                id,
                fps
            )
            VALUES
        "));

        for (idx, m) in data.iter().enumerate() {
            sql.push(format!("(
                '{video_uuid}',
                {res_x},
                {res_y},
                {min_bitrate},
                {avg_bitrate},
                {max_bitrate},
                '{id}',
                {fps}
            )",
                video_uuid=vod_uuid,
                res_x=m.res_x,
                res_y=m.res_y,
                min_bitrate=m.min_bitrate,
                avg_bitrate=m.avg_bitrate,
                max_bitrate=m.max_bitrate,
                id=m.id,
                fps=m.fps
            ));

            if idx != data.len() - 1 {
                sql.push(String::from(","))
            }
        }
        sqlx::query(&sql.join("")).execute(tx).await?;
        Ok(())
    }

    pub async fn fastify_vod(&self, vod_uuid: &Uuid) -> Result<(), squadov_common::SquadOvError> {
        // Simple 4 step process:
        // 1) Download the video from the VOD manager.
        // 2) Convert the video using the vod.fastify module.
        // 3) Re-upload the video using the VOD manager.
        // 4) Mark the video as being "fastified" (I really need a better word).

        let input_filename = NamedTempFile::new()?.into_temp_path();
        let output_filename = NamedTempFile::new()?.into_temp_path();

        // TODO: Remove hard-coded stuff from this.
        log::info!("Download VOD - {}", vod_uuid);
        self.vod.download_vod_to_path(&squadov_common::VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("video.mp4"),
        }, &input_filename).await?;

        log::info!("Fastify Mp4 - {}", vod_uuid);
        vod::fastify::fastify_mp4(&input_filename, &output_filename).await?;

        log::info!("Upload Fastify VOD - {}", vod_uuid);
        self.vod.upload_vod_from_file(&squadov_common::VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("fastify.mp4"),
        }, &output_filename).await?;

        log::info!("Mark DB Fastify (Begin) - {}", vod_uuid);
        let mut tx = self.pool.begin().await?;
        log::info!("Mark DB Fastify (Query) - {}", vod_uuid);
        self.mark_vod_as_fastify(&mut tx, vod_uuid).await?;
        log::info!("Mark DB Fastify (Commit) - {}", vod_uuid);
        tx.commit().await?;

        log::info!("Finish Fastify - {}", vod_uuid);
        Ok(())
    }

    async fn mark_vod_as_fastify(&self, tx : &mut Transaction<'_, Postgres>, vod_uuid: &Uuid) -> Result<(), squadov_common::SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.vod_metadata
            SET has_fastify = true
            WHERE video_uuid = $1
            ",
            vod_uuid
        )
            .execute(tx)
            .await?;
        Ok(())
    }
}

pub async fn associate_vod_handler(path: web::Path<VodAssociatePathInput>, data : web::Json<super::VodAssociateBodyInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let data = data.into_inner();
    if path.video_uuid != data.association.video_uuid {
        return Err(squadov_common::SquadOvError::BadRequest);
    }

    // If the current user doesn't match the UUID passed in the association then reject the request.
    // We could potentially force the association to contain the correct user UUID but in reality
    // the user *should* know their own user UUID.
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };
    
    let input_user_uuid = match data.association.user_uuid {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    if input_user_uuid != session.user.uuid {
        return Err(squadov_common::SquadOvError::Unauthorized);
    }

    let mut tx = app.pool.begin().await?;
    app.associate_vod(&mut tx, &data.association).await?;
    app.bulk_add_video_metadata(&mut tx, &data.association.video_uuid, &[data.metadata]).await?;
    tx.commit().await?;

    // Note that we don't want to spawn a task directly here to "fastify" the VOD
    // because it does take a significant amount of memory/disk space to do so.
    // So we toss it to the local job queue so we can better limit the amount of resources we end up using.
    app.vod_fastify_jobs.enqueue(VodFastifyJob{
        video_uuid: data.association.video_uuid.clone(),
        app: app.get_ref().clone(),
        session_uri: data.session_uri,
    })?;

    return Ok(HttpResponse::Ok().finish());
}

pub async fn create_vod_destination_handler(data : web::Json<VodCreateDestinationUriInput>, app : web::Data<Arc<api::ApiApplication>>, request: HttpRequest) -> Result<HttpResponse, squadov_common::SquadOvError> {
    // First we need to make sure this vod UUID is available in the database before
    // giving the user a path to upload the file.
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };
    
    app.reserve_vod_uuid(&data.video_uuid, session.user.id).await?;

    let path = app.vod.get_segment_upload_uri(&squadov_common::VodSegmentId{
        video_uuid: data.video_uuid.clone(),
        quality: String::from("source"),
        segment_name: String::from("video.mp4")
    }).await?;
    Ok(HttpResponse::Ok().json(&path))
}