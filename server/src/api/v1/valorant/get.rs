use squadov_common::{
    riot::db,
    SquadOvError,
};
use crate::api;
use crate::api::v1::VodAssociation;
use actix_web::{web, HttpResponse};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use std::vec::Vec;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::iter::FromIterator;

#[derive(Deserialize)]
pub struct GetValorantMatchDetailsInput {
    match_id: String,
}

#[derive(Deserialize)]
pub struct GetValorantPlayerMatchMetadataInput {
    match_id: String,
    puuid: String
}

struct RawValorantPlayerMatchMetadata {
    pub match_id: String,
    pub puuid: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>
}

impl api::ApiApplication {
    async fn get_puuids_in_valorant_match_from_user_uuids(&self, match_id: &str, uuids: &[Uuid]) -> Result<Vec<(Uuid, String)>, SquadOvError> {
        let raw = sqlx::query!(
            "
            SELECT u.uuid, vmp.puuid
            FROM squadov.valorant_match_players AS vmp
            INNER JOIN squadov.riot_account_links AS ral
                ON ral.puuid = vmp.puuid
            INNER JOIN squadov.users AS u
                ON u.id = ral.user_id
            WHERE vmp.match_id = $1 AND u.uuid = any($2)
            ",
            match_id,
            uuids,
        )
            .fetch_all(&*self.pool)
            .await?;
        Ok(raw.into_iter().map(|x| {
            (x.uuid, x.puuid)
        }).collect())
    }

    async fn get_valorant_player_match_metadata(&self, match_id: &str, puuid: &str)  -> Result<Option<super::ValorantPlayerMatchMetadata>, squadov_common::SquadOvError> {
        match sqlx::query_as!(
            RawValorantPlayerMatchMetadata,
            r#"
            SELECT *
            FROM squadov.valorant_player_match_metadata
            WHERE match_id = $1
                AND puuid = $2
            "#,
            match_id,
            puuid
        )
            .fetch_optional(&*self.pool)
            .await?
        {
            Some(x) => Ok(Some(super::ValorantPlayerMatchMetadata{
                match_id: x.match_id,
                puuid: x.puuid,
                start_time: x.start_time,
                end_time: x.end_time,
                rounds: sqlx::query_as!(
                    super::ValorantPlayerRoundMetadata,
                    r#"
                    SELECT *
                    FROM squadov.valorant_player_round_metadata
                    WHERE match_id = $1
                        AND puuid = $2
                    "#,
                    match_id,
                    puuid
                )
                    .fetch_all(&*self.pool)
                    .await?,
            })),
            None => Ok(None)
        }
    }
}

pub async fn get_valorant_match_details_handler(data : web::Path<GetValorantMatchDetailsInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let match_data = db::get_valorant_match(&*app.pool, &data.match_id).await?;
    Ok(HttpResponse::Ok().json(&match_data))
}

pub async fn get_valorant_player_match_metadata_handler(data: web::Path<GetValorantPlayerMatchMetadataInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    let metadata = match app.get_valorant_player_match_metadata(&data.match_id, &data.puuid).await? {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::NotFound)
    };
    Ok(HttpResponse::Ok().json(&metadata))
}

#[derive(Deserialize)]
pub struct ValorantMatchUserVodAccessInput {
    pub match_id: String,
    pub user_id: i64
}

#[derive(Serialize)]
struct ValorantUserAccessibleVodOutput {
    pub vods: Vec<VodAssociation>,
    #[serde(rename="userMapping")]
    pub user_mapping: HashMap<Uuid, String>
}

pub async fn get_valorant_match_user_accessible_vod_handler(data: web::Path<ValorantMatchUserVodAccessInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    // We need to get a list of VOD information for each player in the match filtered by
    // the users that the input user has access to. We assume that the ACL on the user_id is
    // appropriate and taken care of externally.
    let match_uuid = db::get_valorant_match_uuid_if_exists(&*app.pool, &data.match_id).await?;
    if match_uuid.is_none() {
        return Err(SquadOvError::NotFound);
    }
    let match_uuid = match_uuid.unwrap();

    let vods = app.find_accessible_vods_in_match_for_user(&match_uuid, data.user_id).await?;

    // Note that for each VOD we also need to figure out the mapping from user uuid to puuid.
    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    Ok(HttpResponse::Ok().json(ValorantUserAccessibleVodOutput{
        vods,
        user_mapping: HashMap::from_iter(app.get_puuids_in_valorant_match_from_user_uuids(&data.match_id, &user_uuids).await?)
    }))
}