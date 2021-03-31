use crate::api;
use squadov_common::{
    SquadOvError,
    SquadOvGames,
    encrypt::{
        squadov_decrypt,
    },
    access::AccessTokenRequest,
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
    thumbnail_width: Option<String>,
    thumbnail_height: Option<String>,
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

    let token = squadov_common::access::find_encrypted_access_token_from_id(&*app.pool, &share_token).await?;
    let req = squadov_decrypt(token, &app.config.squadov.share_key)?;

    let access = serde_json::from_slice::<AccessTokenRequest>(&req.data)?;
    let user = app.users.get_stored_user_from_uuid(&access.user_uuid, &*app.pool).await?.ok_or(SquadOvError::NotFound)?;

    // Only two possible objects can be shared at the moment: clips and matches.
    // In both cases, we only care to return video when the VOD/Clip in question
    // has been fastified. Otherwise it's just a fancy link.
    let mut resp = OEmbedResponse::default();

    if let Some(clip_uuid) = &access.clip_uuid {
        let clips = app.get_vod_clip_from_clip_uuids(&[clip_uuid.clone()], user.id).await?;
        let clip = clips.first().ok_or(SquadOvError::NotFound)?;
        resp.title = Some(clip.title.clone());
        resp.author_name = Some(clip.clipper.clone());
    } else if let Some(match_uuid) = &access.match_uuid {
        let base_matches = app.get_recent_base_matches(&[match_uuid.clone()], user.id).await?;
        let recent_matches = app.get_recent_matches_from_uuids(&base_matches).await?;
        let m = recent_matches.first().ok_or(SquadOvError::NotFound)?;

        resp.author_name = Some(m.base.username.clone());
        if let Some(aimlab) = &m.aimlab_task {
            resp.title = Some(format!(
                "Aim Lab :: {task} :: {score} [{tm}]",
                task=&aimlab.task_name,
                score=aimlab.score,
                tm=aimlab.create_date.format("%Y%m%d %H:%M:%S").to_string(),
            ));
        } else if let Some(lol) = &m.lol_match {
            resp.title = Some(format!(
                "League of Legends :: {mode} {win} [{tm}]",
                mode=&lol.game_mode,
                win=&lol.win_loss(),
                tm=lol.game_creation.format("%Y%m%d %H:%M:%S").to_string()
            ));
        } else if let Some(tft) = &m.tft_match {
            resp.title = Some(format!(
                "Teamfight Tactics :: {place} [{tm}]",
                place=tft.placement,
                tm=tft.game_datetime.format("%Y%m%d %H:%M:%S").to_string()
            ));
        } else if let Some(val) = &m.valorant_match {
            resp.title = Some(format!(
                "Valorant ({or}-{tr}, {win}) :: {k}/{d}/{a} [{tm}]",
                or=val.rounds_won,
                tr=val.rounds_lost,
                win=if val.won { String::from("Win") } else { String::from("Loss") },
                k=val.kills,
                d=val.deaths,
                a=val.assists,
                tm=val.server_start_time_utc.map(|x| {
                    x.format("%Y%m%d %H:%M:%S").to_string()
                }).unwrap_or(String::from("Unknown"))
            ));
        } else if let Some(wow_challenge) = &m.wow_challenge {
            resp.title = Some(format!(
                "WoW Keystone {instance}, +{lvl} {win} :: [{tm}]",
                instance=&wow_challenge.challenge_name,
                lvl=wow_challenge.keystone_level,
                win=if wow_challenge.success { String::from("Success") } else { String::from("Failure") },
                tm=wow_challenge.finish_time.map(|x| {
                    x.format("%Y%m%d %H:%M:%S").to_string()
                }).unwrap_or(String::from("Unknown"))
            ));
        } else if let Some(wow_encounter) = &m.wow_encounter {
            resp.title = Some(format!(
                "WoW Encounter {nm} {win} :: [{tm}]",
                nm=&wow_encounter.encounter_name,
                win=if wow_encounter.success { String::from("Success") } else { String::from("Failure") },
                tm=wow_encounter.finish_time.map(|x| {
                    x.format("%Y%m%d %H:%M:%S").to_string()
                }).unwrap_or(String::from("Unknown"))
            ));
        } else if let Some(wow_arena) = &m.wow_arena {
            resp.title = Some(format!(
                "WoW Arena {typ} {win} :: [{tm}]",
                typ=&wow_arena.arena_type,
                win=if wow_arena.success { String::from("Success") } else { String::from("Failure") },
                tm=wow_arena.finish_time.map(|x| {
                    x.format("%Y%m%d %H:%M:%S").to_string()
                }).unwrap_or(String::from("Unknown"))
            ));
        } else if m.base.game == SquadOvGames::Hearthstone {
            resp.title = Some(String::from("Hearthstone Match"));
        } else {
            return Err(SquadOvError::NotFound);
        }
    }

    if let Some(video_uuid) = &access.video_uuid {
        let vod_metadata_map = app.get_vod_quality_options(&[video_uuid.clone()]).await?;
        let vod_metadata = vod_metadata_map.get(&video_uuid).ok_or(SquadOvError::NotFound)?.first().ok_or(SquadOvError::NotFound)?;

        if vod_metadata.has_fastify {
            resp.otype = String::from("video");

            let aspect_ratio = (vod_metadata.res_x as f32) / (vod_metadata.res_y as f32);
            let mut video_height: i32 = params.maxheight.unwrap_or(300);
            let mut video_width: i32 = (video_height as f32 * aspect_ratio).floor() as i32;

            if let Some(maxwidth) = params.maxwidth {
                if video_width > maxwidth {
                    video_width = maxwidth;
                    video_height = (video_width as f32 / aspect_ratio).floor() as i32;
                }
            }
            
            resp.width = Some(video_width);
            resp.height = Some(video_height);
            resp.html = Some(format!(
                r#"<iframe src="{src}" width="{width}" height="{height}" frameborder="0" title="{title}" webkitallowfullscreen mozallowfullscreen allowfullscreen></iframe>"#,
                src=format!(
                    "{host}/player/{video_uuid}?share={share_token}",
                    host=&app.config.squadov.app_url,
                    video_uuid=&video_uuid,
                    share_token=&share_token,
                ),
                width=video_width,
                height=video_height,
                title=resp.title.clone().unwrap_or(String::from("SquadOV - Shared Content")),
            ));
        }
    }

    Ok(HttpResponse::Ok().json(resp))
}