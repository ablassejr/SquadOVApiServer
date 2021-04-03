use crate::api;
use squadov_common::{
    SquadOvError,
};
use actix_web::{web, HttpResponse};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use url::Url;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct OEmbedParams {
    url: String,
    maxwidth: Option<i32>,
    maxheight: Option<i32>,
    format: Option<String>
}

#[derive(Serialize)]
pub struct OEmbedResponse {
    #[serde(rename="type")]
    otype: String,
    version: String,
    title: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    provider_name: Option<String>,
    provider_url: Option<String>,
    cache_age: Option<i32>,
    thumbnail_url: Option<String>,
    thumbnail_width: Option<i32>,
    thumbnail_height: Option<i32>,
    html: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
}

impl Default for OEmbedResponse {
    fn default() -> Self {
        Self {
            otype: String::from("link"),
            version: String::from("1.0"),
            title: None,
            author_name: None,
            author_url: None,
            provider_name: Some(String::from("SquadOV")),
            provider_url: Some(String::from("https://www.squadov.gg")),
            cache_age: Some(3600),
            thumbnail_url: None,
            thumbnail_width: None,
            thumbnail_height: None,
            html: None,
            width: None,
            height: None,
        }
    }
}

pub async fn oembed_handler(app : web::Data<Arc<api::ApiApplication>>, params: web::Query<OEmbedParams>) -> Result<HttpResponse, SquadOvError> {
    if let Some(format) = &params.format {
        if format != "json" {
            return Ok(HttpResponse::NotImplemented().finish());
        }
    }

    let url = Url::parse(&params.url).map_err(|_x| { SquadOvError::NotFound })?;
    let app_url = Url::parse(&app.config.squadov.app_url).map_err(|_x| { SquadOvError::NotFound })?;
    if url.origin() != app_url.origin() {
        return Ok(HttpResponse::NotFound().finish())
    }

    // At the moment the only thing we care to support OEmbed for is the share URLs which is just at
    // ${APP_URL}/share/${SHARE_TOKEN}.
    let share_token = {
        if let Some(path_split) = url.path_segments() {
            let mut token: Option<Uuid> = None;
            let mut found_token_indicator = false;
            for x in path_split {
                if x == "share" {
                    found_token_indicator = true;
                } else if found_token_indicator {
                    token = Some(Uuid::parse_str(x)?);
                    break;
                }
            }
            token
        } else {
            None
        }
    }.ok_or(SquadOvError::NotFound)?;

    let metadata = app.get_share_meta_data(&share_token, params.maxwidth, params.maxheight).await?;
    let mut resp = OEmbedResponse::default();
    resp.title = Some(metadata.meta_title);
    resp.author_name = Some(metadata.meta_username);
    resp.otype = metadata.oembed_type;

    if metadata.meta_has_video {
        resp.html = Some(format!(
            r#"<iframe src="{src}" width="{width}" height="{height}" frameborder="0" title="{title}" webkitallowfullscreen mozallowfullscreen allowfullscreen></iframe>"#,
            src=metadata.meta_player.unwrap_or(String::from("#")),
            width=metadata.meta_video_width.unwrap_or(480),
            height=metadata.meta_video_height.unwrap_or(240),
            title=resp.title.clone().unwrap_or(String::from("SquadOV - Shared Content")),
        ));
        resp.width = metadata.meta_video_width;
        resp.height = metadata.meta_video_height;
    }

    if metadata.meta_has_thumbnail {
        resp.thumbnail_url = metadata.meta_thumbnail;
        resp.thumbnail_width = metadata.meta_thumbnail_width;
        resp.thumbnail_height = metadata.meta_thumbnail_height;
    }

    Ok(HttpResponse::Ok().json(resp))
}