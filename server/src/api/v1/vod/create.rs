use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use serde::{Deserialize};
use uuid::Uuid;
use sqlx::{Transaction, Postgres, Executor};
use squadov_common::{
    SquadOvError,
    vod::{
        VodAssociation,
        VodDestination,
    },
    storage::CloudStorageLocation,
};

#[derive(Deserialize)]
pub struct VodCreateDestinationUriInput {
    #[serde(rename="videoUuid")]
    video_uuid: Uuid,
    #[serde(rename="containerFormat")]
    container_format: String,
}

#[derive(Deserialize)]
pub struct VodAssociatePathInput {
    video_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct VodAssociateBodyInput {
    association: VodAssociation,
    metadata: squadov_common::VodMetadata,
    #[serde(rename="sessionUri")]
    session_uri: Option<String>,
}

impl api::ApiApplication {
    pub async fn associate_vod(&self, tx : &mut Transaction<'_, Postgres>, assoc : &VodAssociation) -> Result<(), SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                UPDATE squadov.vods
                SET match_uuid = $1,
                    user_uuid = $2,
                    start_time = $3,
                    end_time = $4,
                    is_local = $5
                WHERE video_uuid = $6
                ",
                assoc.match_uuid,
                assoc.user_uuid,
                assoc.start_time,
                assoc.end_time,
                assoc.is_local,
                assoc.video_uuid,
            )
        ).await?;
        Ok(())
    }

    pub async fn reserve_vod_uuid<'a, T>(&self, ex: T, vod_uuid: &Uuid, container_format: &str, user_id: i64, is_clip: bool) -> Result<(), SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.vods (video_uuid, raw_container_format, user_uuid, is_clip)
            SELECT $1, $2, u.uuid, $4
            FROM squadov.users AS u
            WHERE u.id = $3
            ",
            vod_uuid,
            container_format,
            user_id,
            is_clip,
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    pub async fn bulk_add_video_metadata(&self, tx : &mut Transaction<'_, Postgres>, vod_uuid: &Uuid, data: &[squadov_common::VodMetadata]) -> Result<(), SquadOvError> {
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

    pub async fn get_vod_destination(&self, video_uuid: &Uuid, container_format: &str) -> Result<VodDestination, SquadOvError> {
        let extension = squadov_common::container_format_to_extension(container_format);

        let bucket = self.vod.get_bucket_for_location(CloudStorageLocation::Global).ok_or(SquadOvError::InternalError(String::from("No global storage location configured for VOD storage.")))?;
        let manager = self.get_vod_manager(&bucket).await?;

        let path = manager.get_segment_upload_uri(&squadov_common::VodSegmentId{
            video_uuid: video_uuid.clone(),
            quality: String::from("source"),
            segment_name: format!("video.{}", &extension),
        }).await?;
        Ok(
            VodDestination{
                url: path,
                bucket,
            }
        )
    }
}

pub async fn associate_vod_handler(path: web::Path<VodAssociatePathInput>, data : web::Json<super::VodAssociateBodyInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let data = data.into_inner();
    if path.video_uuid != data.association.video_uuid {
        return Err(SquadOvError::BadRequest);
    }

    // If the current user doesn't match the UUID passed in the association then reject the request.
    // We could potentially force the association to contain the correct user UUID but in reality
    // the user *should* know their own user UUID.
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };
    
    let input_user_uuid = match data.association.user_uuid {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    if input_user_uuid != session.user.uuid {
        return Err(SquadOvError::Unauthorized);
    }

    let mut tx = app.pool.begin().await?;
    app.associate_vod(&mut tx, &data.association).await?;

    let metadata_id = data.metadata.id.clone();
    if !data.association.is_local {
        app.bulk_add_video_metadata(&mut tx, &data.association.video_uuid, &[data.metadata]).await?;
    }
    tx.commit().await?;

    // Note that we don't want to spawn a task directly here to "fastify" the VOD
    // because it does take a significant amount of memory/disk space to do so.
    // So we toss it to the local job queue so we can better limit the amount of resources we end up using.
    if !data.association.is_local {
        app.vod_itf.request_vod_processing(&data.association.video_uuid, &metadata_id, data.session_uri.clone(), true).await?;
    }
    return Ok(HttpResponse::Ok().finish());
}

pub async fn create_vod_destination_handler(data : web::Json<VodCreateDestinationUriInput>, app : web::Data<Arc<api::ApiApplication>>, request: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    // First we need to make sure this vod UUID is available in the database before
    // giving the user a path to upload the file.
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };
    
    let mut tx = app.pool.begin().await?;
    app.reserve_vod_uuid(&mut tx, &data.video_uuid, &data.container_format, session.user.id, false).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(app.get_vod_destination(&data.video_uuid, &data.container_format).await?))
}