use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;
use sqlx;
use std::vec::Vec;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct BulkAddVodMetadataInput {
    video_uuid: Uuid
}

impl api::ApiApplication {
    pub async fn bulk_add_video_metadata(&self, vod_uuid: &Uuid, data: &[common::VodMetadata]) -> Result<(), common::SquadOvError> {
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
                fname
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
                '{fname}'
            )",
                video_uuid=vod_uuid,
                res_x=m.res_x,
                res_y=m.res_y,
                min_bitrate=m.min_bitrate,
                avg_bitrate=m.avg_bitrate,
                max_bitrate=m.max_bitrate,
                id=m.id,
                fname=m.fname
            ));

            if idx != data.len() - 1 {
                sql.push(String::from(","))
            }
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query(&sql.join("")).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

pub async fn bulk_add_video_metadata_handler(data : web::Json<Vec<common::VodMetadata>>, inp : web::Path<BulkAddVodMetadataInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, common::SquadOvError> {
    app.bulk_add_video_metadata(&inp.video_uuid, &data).await?;
    Ok(HttpResponse::Ok().finish())
}