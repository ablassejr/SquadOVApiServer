use squadov_common::{
    SquadOvError,
    riot::db,
};
use crate::api;
use crate::api::v1::VodAssociation;
use actix_web::{web, HttpResponse};
use std::sync::Arc;
use super::LolMatchUserInput;
use serde::Serialize;
use uuid::Uuid;
use std::iter::FromIterator;
use std::collections::HashMap;

#[derive(Serialize)]
struct LolUserAccessibleVodOutput {
    pub vods: Vec<VodAssociation>,
    #[serde(rename="userMapping")]
    pub user_mapping: HashMap<Uuid, i32>
}

pub async fn get_lol_match_handler(data : web::Path<super::LolMatchInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let lol_match = db::get_lol_match(&*app.pool, &data.match_uuid).await?;
    Ok(HttpResponse::Ok().json(&lol_match))
}

pub async fn get_lol_match_user_accessible_vod_handler(data: web::Path<LolMatchUserInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let vods = app.find_accessible_vods_in_match_for_user(&data.match_uuid, data.user_id).await?;

    // Note that for each VOD we also need to figure out the mapping from user uuid to participant ID.
    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    Ok(HttpResponse::Ok().json(LolUserAccessibleVodOutput{
        vods,
        user_mapping: HashMap::from_iter(db::get_participant_ids_in_lol_match_from_user_uuids(&*app.pool, &data.match_uuid, &user_uuids).await?)
    }))
}