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
    },
};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use std::collections::{HashSet, HashMap};

pub struct Match {
    pub uuid : Uuid
}

pub struct MatchCollection {
    pub uuid: Uuid
}

struct RawRecentMatchData {
    video_uuid: Uuid,
    match_uuid: Uuid,
    user_uuid: Uuid,
    tm: DateTime<Utc>,
    username: String,
    user_id: i64
}

impl api::ApiApplication {

    async fn get_recent_base_matches_for_user(&self, user_id: i64, start: i64, end: i64) -> Result<Vec<RawRecentMatchData>, SquadOvError> {
        Ok(
            sqlx::query_as!(
                RawRecentMatchData,
                r#"
                SELECT DISTINCT
                    v.video_uuid AS "video_uuid!",
                    v.match_uuid AS "match_uuid!",
                    v.user_uuid AS "user_uuid!",
                    v.end_time AS "tm!",
                    ou.username AS "username!",
                    ou.id AS "user_id!"
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
                WHERE u.id = $1
                    AND v.match_uuid IS NOT NULL
                    AND v.user_uuid IS NOT NULL
                    AND v.start_time IS NOT NULL
                    AND v.end_time IS NOT NULL
                ORDER BY v.end_time DESC
                LIMIT $2 OFFSET $3
                "#,
                user_id,
                end - start,
                start
            )
                .fetch_all(&*self.pool)
                .await?
        )
    }

}

pub async fn get_recent_matches_for_me_handler(app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest, query: web::Query<api::PaginationParameters>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    let raw_base_matches = app.get_recent_base_matches_for_user(session.user.id, query.start, query.end).await?;

    // First grab all the relevant VOD manifests using all the unique VOD UUID's.
    let mut vod_manifests = app.get_vod(&raw_base_matches.iter().map(|x| { x.video_uuid.clone() }).collect::<Vec<Uuid>>()).await?;

    // Now we need to grab the match summary for each of the matches. Note that this will span a multitude of games
    // so we need to bulk grab as much as possible to reduce the # of trips to the DB.
    let match_uuids = raw_base_matches.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
    let match_player_pairs = raw_base_matches.iter().map(|x| {
        MatchPlayerPair{
            match_uuid: x.match_uuid.clone(),
            player_uuid: x.user_uuid.clone(),
        }
    }).collect::<Vec<MatchPlayerPair>>();
    
    let aimlab_tasks = app.list_aimlab_matches_for_uuids(&match_uuids).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, AimlabTask>>();
    let lol_matches = db::list_lol_match_summaries_for_uuids(&*app.pool, &match_uuids).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, LolPlayerMatchSummary>>();
    let wow_encounters = app.list_wow_encounter_for_uuids(&match_uuids).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, WoWEncounter>>();
    let wow_challenges = app.list_wow_challenges_for_uuids(&match_uuids).await?.into_iter().map(|x| { (x.match_uuid.clone(), x)}).collect::<HashMap<Uuid, WoWChallenge>>();
    // TFT and Valorant are different because the match summary is player dependent.
    let tft_match_uuids: HashSet<Uuid> = db::filter_tft_match_uuids(&*app.pool, &match_uuids).await?.into_iter().collect();
    let mut tft_matches = db::list_tft_match_summaries_for_uuids(&*app.pool, &match_player_pairs)
        .await?
        .into_iter()
        .map(|x| {
            ((x.match_uuid.clone(), x.user_uuid.clone()), x)
        })
        .collect::<HashMap<(Uuid, Uuid), TftPlayerMatchSummary>>();
    let mut valorant_matches = db::list_valorant_match_summaries_for_uuids(&*app.pool, &match_player_pairs)
        .await?
        .into_iter()
        .map(|x| {
            ((x.match_uuid.clone(), x.user_uuid.clone()), x)
        })
        .collect::<HashMap<(Uuid, Uuid), ValorantPlayerMatchSummary>>();
    
    let expected_total = query.end - query.start;
    let got_total = raw_base_matches.len() as i64;
    
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(raw_base_matches.into_iter().map(|x| {
        // Aim Lab, LoL, and WoW match data can be shared across multiple users hence we can't remove any
        // data from the hash maps. TFT and Valorant summary data is player specific hence why it can be removed.
        let key_pair = (x.match_uuid.clone(), x.user_uuid.clone());
        let aimlab_task = aimlab_tasks.get(&x.match_uuid);
        let lol_match = lol_matches.get(&x.match_uuid);
        let tft_match = tft_matches.remove(&key_pair);
        let valorant_match = valorant_matches.remove(&key_pair);
        let wow_encounter = wow_encounters.get(&x.match_uuid);
        let wow_challenge = wow_challenges.get(&x.match_uuid);

        Ok(RecentMatch {
            base: BaseRecentMatch{
                match_uuid: x.match_uuid.clone(),
                tm: x.tm,
                game: if aimlab_task.is_some() {
                    SquadOvGames::AimLab
                } else if lol_match.is_some() {
                    SquadOvGames::LeagueOfLegends
                // We require an additional check for Tft match UUIDs because there's a possibility that the 
                // user didn't actually finish the match yet in which case the match UUID exists but the match
                // details don't.
                } else if tft_match.is_some() || tft_match_uuids.contains(&x.match_uuid) {
                    SquadOvGames::TeamfightTactics
                } else if valorant_match.is_some() {
                    SquadOvGames::Valorant
                } else if wow_encounter.is_some() || wow_challenge.is_some() {
                    SquadOvGames::WorldOfWarcraft
                } else {
                    SquadOvGames::Hearthstone
                },
                vod: vod_manifests.remove(&x.video_uuid).ok_or(SquadOvError::InternalError(String::from("Failed to find expected VOD manifest.")))?,
                username: x.username,
                user_id: x.user_id,
            },
            aimlab_task: aimlab_task.cloned(),
            lol_match: lol_match.cloned(),
            tft_match,
            valorant_match,
            wow_challenge: wow_challenge.cloned(),
            wow_encounter: wow_encounter.cloned(),
        })
    }).collect::<Result<Vec<RecentMatch>, SquadOvError>>()?, &req, &query, expected_total == got_total)?)) 
}