use crate::api;
use crate::api::auth::SquadOVSession;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use squadov_common::{SquadOvError, VodSegmentId};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ClipPathInput {
    video_uuid: Uuid,
}

#[derive(Deserialize)]
pub struct ClipBodyInput {
    title: String,
    description: String,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ClipResponse {
    uuid: Uuid,
    upload_path: String,
}

impl api::ApiApplication {
    async fn create_clip_for_vod(&self, vod_uuid: &Uuid, user_id: i64, title: &str, description: &str) -> Result<ClipResponse, SquadOvError> {
        let clip_uuid = Uuid::new_v4();

        self.reserve_vod_uuid(&clip_uuid, "mp4", user_id).await?;

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "
            INSERT INTO squadov.vod_clips (
                clip_uuid,
                parent_vod_uuid,
                clip_user_id,
                title,
                description
            )
            VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4
            )
            ",
            vod_uuid,
            user_id,
            title,
            description,
        )
            .execute(&mut tx)
            .await?;
        tx.commit().await?;

        Ok(ClipResponse{
            uuid: clip_uuid.clone(),
            upload_path: self.vod.get_segment_upload_uri(&VodSegmentId{
                video_uuid: clip_uuid.clone(),
                quality: String::from("source"),
                segment_name: String::from("video.mp4"),
            }).await?,
        })
    }
}

pub async fn create_clip_for_vod_handler(path: web::Path<ClipPathInput>, data : web::Json<ClipBodyInput>, app : web::Data<Arc<api::ApiApplication>>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let resp = app.create_clip_for_vod(&path.video_uuid, session.user.id, &data.title, &data.description).await?;
    Ok(HttpResponse::Ok().json(&resp))
}