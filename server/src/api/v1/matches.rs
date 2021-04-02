mod create;

pub use create::*;
use uuid::Uuid;
use crate::api;
use crate::api::auth::SquadOVSession;
use actix_web::{web, HttpResponse, HttpRequest};
use squadov_common::{
    SquadOvError,
    SquadOvGames,
    matches::{RecentMatch, BaseRecentMatch, MatchPlayerPair},
    aimlab::AimlabTask,
    riot::db,
    riot::games::{
        LolPlayerMatchSummary,
        TftPlayerMatchSummary,
        ValorantPlayerMatchSummary,
    },
    wow::{
        WoWEncounter,
        WoWChallenge,
        WoWArena,
    },
    access::AccessTokenRequest,
    encrypt::{
        AESEncryptRequest,
        squadov_encrypt,
        squadov_decrypt,
    },
    stats::StatPermission,
};
use std::sync::Arc;
use chrono::{DateTime, Utc, TimeZone};
use std::collections::{HashMap};
use serde::{Serialize, Deserialize};
use url::Url;
use serde_qs::actix::QsQuery;
use crate::api::v1::FavoriteResponse;
use std::convert::TryFrom;

pub struct Match {
    pub uuid : Uuid
}

pub struct MatchCollection {
    pub uuid: Uuid
}

#[derive(Deserialize,Debug)]
pub struct GenericMatchPathInput {
    match_uuid: Uuid
}

pub struct RawRecentMatchData {
    video_uuid: Uuid,
    match_uuid: Uuid,
    user_uuid: Uuid,
    tm: DateTime<Utc>,
    username: String,
    user_id: i64,
    favorite_reason: Option<String>,
    is_watchlist: bool,
    game: SquadOvGames,
}


#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RecentMatchQuery {
    pub games: Option<Vec<SquadOvGames>>,
    pub squads: Option<Vec<i64>>,
    pub users: Option<Vec<i64>>,
    pub time_start: Option<i64>,
    pub time_end: Option<i64>,
    pub only_favorite: bool,
    pub only_watchlist: bool,
}

fn filter_recent_match_data_by_game(data: &[RawRecentMatchData], game: SquadOvGames) -> Vec<&RawRecentMatchData> {
    data
        .iter()
        .filter(|x| {
            x.game == game
        })
        .collect()
}

fn recent_match_data_uuids(data: &[&RawRecentMatchData]) -> Vec<Uuid> {
    data.iter().map(|x| { x.match_uuid.clone() }).collect()
}

fn recent_match_data_uuid_pairs(data: &[&RawRecentMatchData]) -> Vec<MatchPlayerPair> {
    data.iter().map(|x| {
        MatchPlayerPair{
            match_uuid: x.match_uuid.clone(),
            player_uuid: x.user_uuid.clone(),
        }
    }).collect()
}

impl api::ApiApplication {

    pub async fn get_recent_base_matches(&self, uuids: &[Uuid], user_id: i64) -> Result<Vec<RawRecentMatchData>, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT DISTINCT
                    v.video_uuid AS "video_uuid!",
                    v.match_uuid AS "match_uuid!",
                    v.user_uuid AS "user_uuid!",
                    v.end_time AS "tm!",
                    ou.username AS "username!",
                    ou.id AS "user_id!",
                    ufm.reason AS "favorite_reason?",
                    uwv.video_uuid IS NOT NULL AS "is_watchlist!",
                    m.game AS "game!"
                FROM squadov.users AS u
                LEFT JOIN squadov.squad_role_assignments AS sra
                    ON sra.user_id = u.id
                LEFT JOIN squadov.squad_role_assignments AS ora
                    ON ora.squad_id = sra.squad_id
                INNER JOIN squadov.users AS ou
                    ON ou.id = ora.user_id
                        OR ou.id = u.id
                INNER JOIN squadov.vods AS v
                    ON v.user_uuid = ou.uuid
                INNER JOIN squadov.matches AS m
                    ON v.match_uuid = m.uuid
                LEFT JOIN squadov.user_favorite_matches AS ufm
                    ON ufm.match_uuid = m.uuid
                        AND ufm.user_id = $2
                LEFT JOIN squadov.user_watchlist_vods AS uwv
                    ON uwv.video_uuid = v.video_uuid
                        AND uwv.user_id = $2
                WHERE v.match_uuid = ANY($1)
                ORDER BY v.end_time DESC
                "#,
                uuids,
                user_id,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| {
                    Ok(RawRecentMatchData {
                        video_uuid: x.video_uuid,
                        match_uuid: x.match_uuid,
                        user_uuid: x.user_uuid,
                        tm: x.tm,
                        username: x.username,
                        user_id: x.user_id,
                        favorite_reason: x.favorite_reason,
                        is_watchlist: x.is_watchlist,
                        game: SquadOvGames::try_from(x.game)?
                    })
                })
                .collect::<Result<Vec<RawRecentMatchData>, SquadOvError>>()?
        )
    }

    async fn get_recent_base_matches_for_user(&self, user_id: i64, start: i64, end: i64, filter: &RecentMatchQuery) -> Result<Vec<RawRecentMatchData>, SquadOvError> {
        let uuids: Vec<Uuid> = sqlx::query!(
            r#"
            SELECT DISTINCT v.match_uuid AS "match_uuid!", v.end_time
            FROM squadov.users AS u
            LEFT JOIN squadov.squad_role_assignments AS sra
                ON sra.user_id = u.id
            LEFT JOIN squadov.squad_role_assignments AS ora
                ON ora.squad_id = sra.squad_id
            INNER JOIN squadov.users AS ou
                ON ou.id = ora.user_id
                    OR ou.id = u.id
            INNER JOIN squadov.vods AS v
                ON v.user_uuid = ou.uuid
            INNER JOIN squadov.matches AS m
                ON v.match_uuid = m.uuid
            LEFT JOIN squadov.user_favorite_matches AS ufm
                ON ufm.match_uuid = m.uuid
                    AND ufm.user_id = $1
            LEFT JOIN squadov.user_watchlist_vods AS uwv
                ON uwv.video_uuid = v.video_uuid
                    AND uwv.user_id = $1
            WHERE u.id = $1
                AND v.match_uuid IS NOT NULL
                AND v.user_uuid IS NOT NULL
                AND v.start_time IS NOT NULL
                AND v.end_time IS NOT NULL
                AND (CARDINALITY($4::INTEGER[]) = 0 OR m.game = ANY($4))
                AND (CARDINALITY($5::BIGINT[]) = 0 OR sra.squad_id = ANY($5))
                AND (CARDINALITY($6::BIGINT[]) = 0 OR ou.id = ANY($6))
                AND COALESCE(v.end_time >= $7, TRUE)
                AND COALESCE(v.end_time <= $8, TRUE)
                AND (NOT $9::BOOLEAN OR ufm.match_uuid IS NOT NULL)
                AND (NOT $10::BOOLEAN OR uwv.video_uuid IS NOT NULL)
            ORDER BY v.end_time DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            end - start,
            start,
            &filter.games.as_ref().unwrap_or(&vec![]).iter().map(|x| {
                *x as i32
            }).collect::<Vec<i32>>(),
            &filter.squads.as_ref().unwrap_or(&vec![]).iter().map(|x| { *x }).collect::<Vec<i64>>(),
            &filter.users.as_ref().unwrap_or(&vec![]).iter().map(|x| { *x }).collect::<Vec<i64>>(),
            filter.time_start.map(|x| {
                Utc.timestamp_millis(x)
            }),
            filter.time_end.map(|x| {
                Utc.timestamp_millis(x)
            }),
            filter.only_favorite,
            filter.only_watchlist
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { x.match_uuid })
            .collect();
        Ok(self.get_recent_base_matches(&uuids, user_id).await?)
    }

    async fn is_match_favorite_by_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<Option<String>, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                SELECT reason
                FROM squadov.user_favorite_matches
                WHERE match_uuid = $1
                    AND user_id = $2

                "#,
                match_uuid,
                user_id,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| { x.reason })
        )
    }

    async fn add_match_favorite_for_user(&self, match_uuid: &Uuid, user_id: i64, reason: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            INSERT INTO squadov.user_favorite_matches (
                match_uuid,
                user_id,
                reason
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ON CONFLICT DO NOTHING
            "#,
            match_uuid,
            user_id,
            reason,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    async fn remove_match_favorite_for_user(&self, match_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            r#"
            DELETE FROM squadov.user_favorite_matches
            WHERE match_uuid = $1 AND user_id = $2
            "#,
            match_uuid,
            user_id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_recent_matches_from_uuids(&self, raw_base_matches: &[RawRecentMatchData]) -> Result<Vec<RecentMatch>, SquadOvError> {
        // First grab all the relevant VOD manifests using all the unique VOD UUID's.
        let mut vod_manifests = self.get_vod(&raw_base_matches.iter().map(|x| { x.video_uuid.clone() }).collect::<Vec<Uuid>>()).await?;
        
        let aimlab_tasks = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::AimLab);

            if !recent.is_empty() {
                self.list_aimlab_matches_for_uuids(&recent_match_data_uuids(&recent)).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, AimlabTask>>()
            } else {
                HashMap::new()
            }
        };
        let lol_matches = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::LeagueOfLegends);
            if !recent.is_empty() {
                db::list_lol_match_summaries_for_uuids(&*self.pool, &recent_match_data_uuids(&recent)).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, LolPlayerMatchSummary>>()
            } else {
                HashMap::new()
            }
        };
        // TFT, Valorant, and WoW is different because the match summary is player dependent.
        let mut wow_encounters = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::WorldOfWarcraft);
            if !recent.is_empty() {
                self.list_wow_encounter_for_uuids(&recent_match_data_uuid_pairs(&recent)).await?.into_iter().map(|x| { ((x.match_uuid.clone(), x.user_uuid.clone()), x)}).collect::<HashMap<(Uuid, Uuid), WoWEncounter>>()
            } else {
                HashMap::new()
            }
        };
        let mut wow_challenges = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::WorldOfWarcraft);
            if !recent.is_empty() {
                self.list_wow_challenges_for_uuids(&recent_match_data_uuid_pairs(&recent)).await?.into_iter().map(|x| { ((x.match_uuid.clone(), x.user_uuid.clone()), x)}).collect::<HashMap<(Uuid, Uuid), WoWChallenge>>()
            } else {
                HashMap::new()
            }
        };
        let mut wow_arenas = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::WorldOfWarcraft);
            if !recent.is_empty() {
                self.list_wow_arenas_for_uuids(&recent_match_data_uuid_pairs(&recent)).await?.into_iter().map(|x| { ((x.match_uuid.clone(), x.user_uuid.clone()), x)}).collect::<HashMap<(Uuid, Uuid), WoWArena>>()
            } else {
                HashMap::new()
            }
        };
        let mut tft_matches = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::TeamfightTactics);
            if !recent.is_empty() {
                db::list_tft_match_summaries_for_uuids(&*self.pool, &recent_match_data_uuid_pairs(&recent))
                    .await?
                    .into_iter()
                    .map(|x| {
                        ((x.match_uuid.clone(), x.user_uuid.clone()), x)
                    })
                    .collect::<HashMap<(Uuid, Uuid), TftPlayerMatchSummary>>()
            } else {
                HashMap::new()
            }
        };
        let mut valorant_matches = {
            let recent = filter_recent_match_data_by_game(raw_base_matches, SquadOvGames::Valorant);
            if !recent.is_empty() {
                db::list_valorant_match_summaries_for_uuids(&*self.pool, &recent_match_data_uuid_pairs(&recent))
                    .await?
                    .into_iter()
                    .map(|x| {
                        ((x.match_uuid.clone(), x.user_uuid.clone()), x)
                    })
                    .collect::<HashMap<(Uuid, Uuid), ValorantPlayerMatchSummary>>()
            } else {
                HashMap::new()
            }
        };

        Ok(
            raw_base_matches.into_iter().map(|x| {
                // Aim Lab, LoL, and WoW match data can be shared across multiple users hence we can't remove any
                // data from the hash maps. TFT and Valorant summary data is player specific hence why it can be removed.
                let key_pair = (x.match_uuid.clone(), x.user_uuid.clone());
                let aimlab_task = aimlab_tasks.get(&x.match_uuid);
                let lol_match = lol_matches.get(&x.match_uuid);
                let tft_match = tft_matches.remove(&key_pair);
                let valorant_match = valorant_matches.remove(&key_pair);
                let wow_encounter = wow_encounters.remove(&key_pair);
                let wow_challenge = wow_challenges.remove(&key_pair);
                let wow_arena = wow_arenas.remove(&key_pair);
        
                Ok(RecentMatch {
                    base: BaseRecentMatch{
                        match_uuid: x.match_uuid.clone(),
                        tm: x.tm,
                        game: x.game,
                        vod: vod_manifests.remove(&x.video_uuid).ok_or(SquadOvError::InternalError(String::from("Failed to find expected VOD manifest.")))?,
                        username: x.username.clone(),
                        user_id: x.user_id,
                        favorite_reason: x.favorite_reason.clone(),
                        is_watchlist: x.is_watchlist,
                    },
                    aimlab_task: aimlab_task.cloned(),
                    lol_match: lol_match.cloned(),
                    tft_match,
                    valorant_match,
                    wow_challenge,
                    wow_encounter,
                    wow_arena,
                })
            }).collect::<Result<Vec<RecentMatch>, SquadOvError>>()?
        )
    }
}

pub async fn get_recent_matches_for_me_handler(app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest, query: QsQuery<api::PaginationParameters>, filter: QsQuery<RecentMatchQuery>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let raw_base_matches = app.get_recent_base_matches_for_user(session.user.id, query.start, query.end, &filter).await?;
    let matches = app.get_recent_matches_from_uuids(&raw_base_matches).await?;
    
    let expected_total = query.end - query.start;
    let got_total = raw_base_matches.len() as i64;
    
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(&matches, &req, &query, expected_total == got_total)?)) 
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct MatchShareSignatureData {
    full_path: String,
    game: SquadOvGames,
    graphql_stats: Option<Vec<StatPermission>>,
}

pub async fn create_match_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, data: web::Json<MatchShareSignatureData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    // We need to verify that the requesting user is actually a part of the match. Note that we should not
    // check for the presence of a VOD because we should keep it possible to share matches that are VOD-less.
    if !squadov_common::matches::is_user_in_match(&*app.pool, session.user.id, &path.match_uuid, data.game).await? {
        return Err(SquadOvError::Unauthorized);
    }

    // Next we need to generate the share URL for this match. This is dependent on
    // 1) The app domain
    // 2) The game of the match being requested.
    // 3) The user's POV that's being requested.
    // #1 is something the API server should know while #2 is something more along the lines of what
    // the client (user) should know. HOWEVER, we shouldn't fully trust them so we need to examine the
    // URL that they sent to do further verification. Generally the only two things we need to verify are
    // 1) The user ID (should match the session user id)
    // 2) The Riot PUUID (only in certain cases).
    let base_url = format!(
        "{}{}",
        &app.config.cors.domain,
        &data.full_path,
    );
    let parsed_url = Url::parse(&base_url)?;
    for qp in parsed_url.query_pairs() {
        if qp.0 == "userId" {
            if qp.1.parse::<i64>()? != session.user.id {
                return Err(SquadOvError::Unauthorized);
            }
        } else if qp.0 == "puuid" {
            if !db::is_riot_puuid_linked_to_user(&*app.pool, session.user.id, &qp.1).await? {
                return Err(SquadOvError::Unauthorized);
            }
        }
    }

    // If the user already shared this match, reuse that token so we don't fill up our databases with a bunch of useless tokens.
    let mut token = squadov_common::access::find_encrypted_access_token_for_match_user(&*app.pool, &path.match_uuid, session.user.id).await?;

    if token.is_none() {
        // Now that we've verified all these things we can go ahead and return to the user a fully fleshed out
        // URL that can be shared. We enable this by generating an encrypted access token that can be used to imitate 
        // access as this session's user to ONLY this current match UUID (along with an optional VOD UUID if one exists).
        let access_request = AccessTokenRequest{
            full_path: data.full_path.clone(),
            user_uuid: session.user.uuid.clone(),
            match_uuid: Some(path.match_uuid.clone()),
            video_uuid: app.find_vod_from_match_user_id(path.match_uuid.clone(), session.user.id).await?.map(|x| {
                x.video_uuid
            }),
            clip_uuid: None,
            graphql_stats: data.graphql_stats.clone(),
        };

        let encryption_request = AESEncryptRequest{
            data: serde_json::to_vec(&access_request)?,
            aad: session.user.uuid.as_bytes().to_vec(),
        };

        let encryption_token = squadov_encrypt(encryption_request, &app.config.squadov.share_key)?;

        // Store the encrypted token in our database and return to the user a URL with the unique ID and the IV.
        // This way we get a (relatively) shorter URL instead of a giant encrypted blob.
        let mut tx = app.pool.begin().await?;
        let token_id = squadov_common::access::store_encrypted_access_token_for_match_user(&mut tx, &path.match_uuid, session.user.id, &encryption_token).await?;
        tx.commit().await?;

        token = Some(token_id);
    }

    let token = token.ok_or(SquadOvError::InternalError(String::from("Failed to obtain/generate share token.")))?;

    // It could be neat to store some sort of access token ID in our database and allow users to track how
    // many times it was used and be able to revoke it and stuff but I don't think the gains are worth it at
    // the moment. I'd rather have a more distributed version where we toss a URL out there and just let it be
    // valid.
    Ok(HttpResponse::Ok().json(&format!(
        "{}/share/{}",
        &app.config.cors.domain,
        &token,
    )))
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct MatchFavoriteData {
    reason: String,
}

pub async fn favorite_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, data: web::Json<MatchFavoriteData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.add_match_favorite_for_user(&path.match_uuid, session.user.id, &data.reason).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn check_favorite_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let reason = app.is_match_favorite_by_user(&path.match_uuid, session.user.id).await?;
    Ok(HttpResponse::Ok().json(
        FavoriteResponse{
            favorite: reason.is_some(),
            reason,
        }
    ))
}

pub async fn remove_favorite_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    app.remove_match_favorite_for_user(&path.match_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize,Debug)]
pub struct ExchangeShareTokenPath {
    access_token_id: Uuid
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ShareTokenResponse {
    full_path: String,
    key: String,
    uid: i64,
}

pub async fn exchange_access_token_id_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<ExchangeShareTokenPath>) -> Result<HttpResponse, SquadOvError> {
    let token = squadov_common::access::find_encrypted_access_token_from_id(&*app.pool, &path.access_token_id).await?;
    let key = token.to_string();
    let req = squadov_decrypt(token, &app.config.squadov.share_key)?;

    let access = serde_json::from_slice::<AccessTokenRequest>(&req.data)?;
    Ok(HttpResponse::Ok().json(&ShareTokenResponse{
        full_path: access.full_path,
        key,
        uid: app.users.get_stored_user_from_uuid(&access.user_uuid, &*app.pool).await?.ok_or(SquadOvError::NotFound)?.id,
    }))
}