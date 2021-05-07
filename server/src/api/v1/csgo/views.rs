use actix_web::{web, HttpResponse};
use crate::api;
use squadov_common::{
    SquadOvError,
    SquadOvGames,
    csgo::gsi::CsgoGsiMatchState,
    csgo::db,
    steam
};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct CsgoCreateViewPath {
    user_id: i64
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoCreateViewData {
    server: String,
    start_time: DateTime<Utc>,
    map: String,
    mode: String,
}

#[derive(Deserialize,Debug)]
pub struct CsgoViewPath {
    user_id: i64,
    view_uuid: Uuid,
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoViewData {
    stop_time: DateTime<Utc>,
    data: CsgoGsiMatchState,
    local_steam_id: i64,
    demo: Option<String>,
    demo_timestamp: Option<DateTime<Utc>>,
}

pub async fn create_csgo_view_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CsgoCreateViewPath>, data: web::Json<CsgoCreateViewData>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    let view = db::create_csgo_view_for_user(&mut tx, path.user_id, &data.server, &data.start_time, &data.map, &data.mode).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(&view))
}

pub async fn finish_csgo_view_for_user_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CsgoViewPath>, data: web::Json<CsgoViewData>) -> Result<HttpResponse, SquadOvError> {
    let view = db::find_csgo_view(&*app.pool, &path.view_uuid).await?;

    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match db::find_existing_csgo_match(&*app.pool, &view.game_server, &view.start_time, &data.stop_time).await? {
            Some(uuid) => uuid,
            None => {
                let new_match = app.create_new_match(&mut tx, SquadOvGames::Csgo).await?;
                match db::create_csgo_match(&mut tx, &new_match.uuid, &view.game_server, &view.start_time, &data.stop_time).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        SquadOvError::Duplicate => {
                            log::warn!("Caught duplicate CSGO match...retrying!");
                            continue;
                        },
                        _ => return Err(err)
                    }
                };
                new_match.uuid
            }
        };
        db::finish_csgo_view(&mut tx, &path.view_uuid, &match_uuid, &data.stop_time, &data.data).await?;
        steam::link_steam_id_to_user(&mut tx, data.local_steam_id, path.user_id).await?;
        app.steam_itf.request_sync_steam_accounts(&[data.local_steam_id]).await?;

        if let Some(demo) = &data.demo {
            if let Some(demo_timestamp) = &data.demo_timestamp {
                app.csgo_itf.request_parse_csgo_demo_from_url(&path.view_uuid, demo, demo_timestamp).await?;
            }
        }
        
        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }

    Err(SquadOvError::InternalError(String::from("Reached CSGO finish view retry threshold.")))
}