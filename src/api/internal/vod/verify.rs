use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct VerifyVodStreamKeyInput {
    //call: String,
    //addr: String,
    //clientid: i64,
    //app: String,
    //#[serde(rename="flashver")]
    //flash_ver: String,
    //#[serde(rename="swfurl")]
    //swf_url: String,
    //#[serde(rename="tcurl")]
    //tc_url: String,
    //#[serde(rename="pageurl")]
    //page_url: String,
    name: String
}

impl api::ApiApplication {
    pub async fn verify_and_reserve_vod_stream_key(&self, stream_key: &str) -> Result<(), common::SquadOvError> {
        // Split the stream key into the user uuid and video uuid.
        // 1) Verify that the user UUID is valid and points to a legit user.
        // 2) Verify that the video UUID is unique and if it is unique, reserve it. Use the
        //    reserve UUID function to do both as it'll throw a PostgreSQL error if the unique
        //    constraint is not met.
        let stream_key = common::parse_vod_stream_key(stream_key)?;

        if !self.internal_check_user_uuid_exists(&stream_key.user_uuid).await? {
            return Err(common::SquadOvError::NotFound);
        }

        self.reserve_vod_uuid(&stream_key.vod_uuid).await
    }
}

pub async fn verify_and_reserve_vod_stream_key_handler(data : web::Query<VerifyVodStreamKeyInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, common::SquadOvError> {
    app.verify_and_reserve_vod_stream_key(&data.name).await?;
    Ok(HttpResponse::Ok().finish())
}