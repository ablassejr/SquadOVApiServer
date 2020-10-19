use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use m3u8_rs::playlist::{MasterPlaylist, VariantStream, AlternativeMedia, AlternativeMediaType};
use serde::{Serialize, Deserialize};
use std::default::Default;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VodFindFromVideoUuid {
    video_uuid: Uuid,
}

impl api::ApiApplication {
    pub async fn get_vod_quality_options(&self, video_uuid: &Uuid) -> Result<Vec<super::VodVideoQualityOption>, common::SquadOvError> {
        // List variants by querying the storage system (filesystem vs GCS).
        let options = self.vod.get_vod_video_quality_options(video_uuid).await?;
        Ok(options)
    }

    pub async fn get_vod(&self, video_uuid: &Uuid) -> Result<String, common::SquadOvError> {
        return Ok(String::from(""));

        /*
        let mut alts = vec![
            AlternativeMedia{
                media_type: AlternativeMediaType::Audio,
                group_id: String::from("source"),
                name: String::from("Source"),
                uri: Some(String::from("")),
                default: true,
                autoselect: true,
                ..Default::default()
            }
        ];

        let options = self.get_vod_quality_options(video_uuid).await?;

        let mut variants = vec![];
        for o in &options {
            variants.push(
                VariantStream{
                    uri: String::from(""),
                    bandwidth: String::from(""),
                    codecs: Some(String::from("")),
                    alternatives: alts.clone(),
                    ..Default::default()
                }
            )
        }
        let playlist = MasterPlaylist{
            version : 3,
            independent_segments: true,
            variants: variants,
            ..Default::default()
        };



        let mut v : Vec<u8> = Vec::new();
        playlist.write_to(&mut v)?;
        Ok(String::from(std::str::from_utf8(&v)?))
        */
    }
}

pub async fn get_vod_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let m3u8 = app.get_vod(&data.video_uuid).await?;
    Ok(HttpResponse::Ok().json(&m3u8))
}

pub async fn get_vod_quality_handler(data : web::Path<VodFindFromVideoUuid>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let opts = app.get_vod_quality_options(&data.video_uuid).await?;
    Ok(HttpResponse::Ok().json(&opts))
}