mod create;
mod events;

pub use create::*;
pub use events::*;

use uuid::Uuid;
use crate::api;
use crate::api::auth::{SquadOvMachineId, SquadOVSession};
use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use squadov_common::{
    SquadOvError,
    SquadOvGames,
    SquadOvWowRelease,
    games,
    matches::{RecentMatch, self},
    riot::{
        ValorantMatchFilters,
    },
    access::{
        AccessTokenRequest,
        AccessToken,
    },
    encrypt::{
        AESEncryptRequest,
        squadov_encrypt,
        squadov_decrypt,
    },
    stats::StatPermission,
    share,
    share::{
        LinkShareData,
    },
    vod::{
        db as vdb,
    },
    elastic::vod::ESVodDocument,
};
use std::sync::Arc;
use chrono::{Utc, Duration};
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::api::v1::{
    FavoriteResponse,
    UserProfilePath,
    wow::WowListQuery,
};
use elasticsearch_dsl::{Search, Sort, SortOrder, Query};

pub struct Match {
    pub uuid : Uuid
}

pub struct MatchCollection {
    pub uuid: Uuid
}

#[derive(Deserialize,Debug)]
pub struct GenericMatchPathInput {
    pub match_uuid: Uuid
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct GenericWowQuery {
    pub encounters: WowListQuery,
    pub keystones: WowListQuery,
    pub arenas: WowListQuery,
    pub instances: WowListQuery,
}

impl Default for GenericWowQuery {
    fn default() -> Self {
        Self {
            encounters: WowListQuery::default(),
            keystones: WowListQuery::default(),
            arenas: WowListQuery::default(),
            instances: WowListQuery::default(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct RecentMatchGameQuery {
    pub wow: GenericWowQuery,
    pub valorant: ValorantMatchFilters,
}

impl Default for RecentMatchGameQuery {
    fn default() -> Self {
        Self {
            wow: GenericWowQuery::default(),
            valorant: ValorantMatchFilters::default(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct RecentMatchQuery {
    pub games: Option<Vec<SquadOvGames>>,
    pub wow_releases: Option<Vec<SquadOvWowRelease>>,
    pub tags: Option<Vec<String>>,
    // Shared to squads
    pub squads: Option<Vec<i64>>,
    #[serde(default)]
    pub must_match_squads: bool,
    // Recorded by user
    pub users: Option<Vec<i64>>,
    pub time_start: Option<i64>,
    pub time_end: Option<i64>,
    pub only_favorite: bool,
    pub only_watchlist: bool,
    #[serde(default)]
    pub only_profile: bool,
    pub vods: Option<Vec<Uuid>>,
    pub not_vods: Option<Vec<Uuid>>,
    pub matches: Option<Vec<Uuid>>,
    pub filters: RecentMatchGameQuery,
}

impl RecentMatchQuery {
    pub fn to_es_search(&self, user_id: i64, machine_id: Option<&str>, is_clip_term: bool) -> Search {
        let mut q = Query::bool();
        if let Some(games) = self.games.as_ref() {
            q = q.filter(Query::terms("data.game", games.iter().map(|x| *x as i32).collect::<Vec<i32>>()));
        }

        if let Some(wow_releases) = self.wow_releases.as_ref() {
            if !wow_releases.is_empty() {
                let mut wr_query = Query::bool();

                if !wow_releases.is_empty() {
                    wr_query = wr_query.minimum_should_match("1");
                }

                for wr in wow_releases {
                    wr_query = wr_query.should(Query::regexp("data.wow.buildVersion", games::wow_release_to_regex_expression(*wr)));
                }

                q = q.filter(wr_query);
            }
        }

        // If a machine ID is set then we only want to return videos that are stored on the cloud OR are stored on the machine.
        // If no machine ID is set, then for whatever reason, the caller doesn't need this separation (e.g. only interested in the match details).
        if let Some(machine_id) = machine_id {
            q = q.filter(
                Query::bool()
                    .minimum_should_match("1")
                    .should(
                        // Mainly for legacy from pre-transition.
                        // The bool must_not must be outside of the nested or else this query doesn't work.
                        Query::bool()
                            .must_not(
                                Query::nested(
                                    "storageCopiesExact",
                                    Query::exists("storageCopiesExact")
                                )
                            )
                    )
                    .should(
                        Query::nested(
                            "storageCopiesExact",
                            Query::bool()
                                .filter(Query::term("storageCopiesExact.loc", 0))
                        )
                    )
                    .should(
                        Query::nested(
                            "storageCopiesExact",
                            Query::bool()
                                .filter(Query::term("storageCopiesExact.loc", 1))
                                .filter(Query::term("storageCopiesExact.spec", machine_id))
                        )
                    )
            );
        }

        if let Some(tags) = self.tags.as_ref() {
            q = q.filter(Query::terms("tags.tag", tags.clone()));
        }

        {
            let mut sharing_query = Query::bool()
                .should(Query::term("owner.userId", user_id));

            if let Some(squads) = self.squads.as_ref() {
                let inner = Query::bool()
                    .filter(Query::terms("sharing.squads", squads.clone()));

                if self.must_match_squads {
                    sharing_query = sharing_query.filter(inner);
                } else {
                    sharing_query = sharing_query
                        .minimum_should_match("1")
                        .should(inner);
                }
            }

            q = q.filter(sharing_query);
        }

        if let Some(users) = self.users.as_ref() {
            q = q.filter(Query::terms("owner.userId", users.clone()));
        }

        {
            let mut r = Query::range("vod.endTime");
            if let Some(ts) = self.time_start {
                r = r.gte(ts);
            }

            r = r.lte(self.time_end.unwrap_or(Utc::now().timestamp_millis()));
            q = q.filter(r);
        }

        if self.only_favorite {
            q = q.filter(Query::nested(
                "lists.favorites",
                Query::term("lists.favorites.userId", user_id),
            ));
        }

        if self.only_watchlist {
            q = q.filter(Query::term("lists.watchlist", user_id));
        }

        if self.only_profile {
            q = q.filter(Query::term("lists.profiles", user_id));
        }

        if let Some(vods) = self.vods.as_ref() {
            q = q.filter(Query::terms("_id", vods.iter().map(|x| { x.to_hyphenated().to_string() }).collect::<Vec<_>>()));
        }

        if let Some(not_vods) = self.not_vods.as_ref() {
            q = q.must_not(Query::terms("_id", not_vods.iter().map(|x| { x.to_hyphenated().to_string() }).collect::<Vec<_>>()));
        }

        if let Some(matches) = self.matches.as_ref() {
            q = q.filter(Query::terms("data.matchUuid", matches.iter().map(|x| { x.to_hyphenated().to_string() }).collect::<Vec<_>>()));
        }

        let game_filters = vec![
            self.filters.valorant.build_es_query(),
            if self.filters.wow.encounters.enabled {
                self.filters.wow.encounters.build_es_query()
            } else {
                Query::bool()
                    .must_not(Query::exists("data.wow.encounter"))
            },
            if self.filters.wow.arenas.enabled {
                self.filters.wow.arenas.build_es_query()
            } else {
                Query::bool()
                    .must_not(Query::exists("data.wow.arena"))
            },
            if self.filters.wow.keystones.enabled {
                self.filters.wow.keystones.build_es_query()
            } else {
                Query::bool()
                    .must_not(Query::exists("data.wow.challenge"))
            },
            if self.filters.wow.instances.enabled {
                self.filters.wow.instances.build_es_query()
            } else {
                Query::bool()
                    .must_not(Query::exists("data.wow.instance"))
            },
        ];

        {
            let mut gquery = Query::bool();
            for f in game_filters {
                gquery = gquery.filter(f);
            }

            q = q.filter(gquery);
        }
        
        q = q.filter(Query::term("vod.isClip", is_clip_term));
        Search::new().query(q)
    }
}

impl Default for RecentMatchQuery {
    fn default() -> Self {
        Self {
            games: None,
            wow_releases: None,
            tags: None,
            squads: None,
            must_match_squads: false,
            users: None,
            time_start: None,
            time_end: None,
            only_favorite: false,
            only_watchlist: false,
            only_profile: false,
            not_vods: None,
            vods: None,
            matches: None,
            filters: RecentMatchGameQuery::default(),
        }
    }
}

#[derive(Debug)]
pub struct RecentMatchHandle {
    pub match_uuid: Uuid,
    pub user_uuids: Vec<Uuid>,
}

impl api::ApiApplication {
    pub async fn is_user_allowed_to_es_search(&self, user_id: i64) -> Result<bool, SquadOvError> {
        Ok(
            !sqlx::query!(
                r#"
                SELECT disable_es_search AS "disable_es_search!"
                FROM squadov.user_feature_flags
                WHERE user_id = $1
                "#,
                user_id
            )
                .fetch_one(&*self.pool)
                .await?
                .disable_es_search
        )
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

        for v in vdb::find_accessible_vods_in_match_for_user(&*self.pool, match_uuid, user_id, "").await? {
            self.es_itf.request_update_vod_lists(v.video_uuid).await?;
        }
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
        
        for v in vdb::find_accessible_vods_in_match_for_user(&*self.pool, match_uuid, user_id, "").await? {
            self.es_itf.request_update_vod_lists(v.video_uuid).await?;
        }
        Ok(())
    }

    fn generate_access_token_for_recent_match(&self, match_uuid: &Uuid, game: SquadOvGames, user_id: i64, video_uuid: &Uuid) -> Result<String, SquadOvError> {
        let mut paths: Vec<String> = vec![
            format!("/v1/vod/{}", video_uuid),
            format!("/v1/vod/match/{}/user/id/{}", match_uuid, user_id),
        ];

        match game {
            SquadOvGames::AimLab => {
                paths.append(&mut vec![
                    format!("v1/aimlab/user/{}/match/{}/task", user_id, match_uuid),
                ]);
            },
            SquadOvGames::Csgo => {
                paths.append(&mut vec![
                    format!("v1/csgo/user/{}/match/{}", user_id, match_uuid),
                    format!("v1/csgo/match/{}/vods", match_uuid),
                ]);
            },
            SquadOvGames::Hearthstone => {
                paths.append(&mut vec![
                    format!("v1/hearthstone/user/{}/match/{}", user_id, match_uuid),
                    format!("v1/hearthstone/match/{}/vods",match_uuid),
                ]);
            },
            SquadOvGames::LeagueOfLegends => {
                paths.append(&mut vec![
                    format!("v1/lol/match/{}", match_uuid),
                ]);
            },
            SquadOvGames::TeamfightTactics => {
                paths.append(&mut vec![
                    format!("v1/tft/match/{}", match_uuid),
                ]);
            },
            SquadOvGames::Valorant => {
                paths.append(&mut vec![
                    format!("v1/valorant/match/{}", match_uuid),
                ]);
            },
            SquadOvGames::WorldOfWarcraft => {
                paths.append(&mut vec![
                    format!("v1/wow/users/{}/match/{}", user_id, match_uuid),
                    format!("v1/wow/match/{}/users/{}", match_uuid, user_id),
                    format!("v1/wow/match/{}/vods", match_uuid),
                    String::from("v1/wow/characters/armory"),
                ]);
            },
            _ => (),
        }

        Ok(
            AccessToken{
                // Ideally we'd refresh this somehow instead of just granting access for such a large chunk of time.
                expires: Some(Utc::now() + Duration::hours(6)),
                methods: Some(vec![String::from("GET")]),
                paths: Some(paths),
                user_id: Some(user_id),
            }.encrypt(&self.config.squadov.access_key)?
        )
    }
}

pub async fn get_vod_recent_match_handler(app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest, path: web::Path<super::GenericVodPathInput>, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let es_search = RecentMatchQuery{
        vods: Some(vec![path.video_uuid.clone()]),
        ..RecentMatchQuery::default()
    }.to_es_search(session.user.id, None, false)
        .from(0)
        .size(1)
        .sort(vec![
            Sort::new("vod.endTime")
                .order(SortOrder::Desc)
        ]);

    let documents: Vec<ESVodDocument> = app.es_api.search_documents(&app.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?;
    let matches = matches::vod_documents_to_recent_matches(documents, session.user.id, &machine_id.id);

    if matches.is_empty() {
        Err(SquadOvError::NotFound)
    } else {
        Ok(HttpResponse::Ok().json(&matches[0]))
    }
}

async fn get_recent_matches_for_user(user_id: i64, app : web::Data<Arc<api::ApiApplication>>, req: &HttpRequest, query: web::Query<api::PaginationParameters>, mut filter: web::Json<RecentMatchQuery>, needs_access_tokens: bool, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    if needs_access_tokens {
        filter.users = Some(vec![user_id]);
    }

    let available_user_squads: HashSet<i64> = app.get_user_squads(session.user.id).await?.into_iter().map(|x| { x.squad.id }).collect();
    if let Some(squad_filter) = &filter.squads {
        filter.squads = Some(squad_filter.iter().filter(|x| { available_user_squads.contains(x) }).map(|x| { *x }).collect());
        filter.must_match_squads = true;
    } else {
        filter.squads = Some(available_user_squads.into_iter().collect());
        filter.must_match_squads = false;
    }

    // We need to keep querying VODs until we receive the number of matches the user wants (or there's nothing left).
    // I'm going to make the assumption here that querying ElasticSearch multiple times is better than running aggregation queries -
    // in fact I'm not even sure we can even effectively use aggregation queries to accomplish what I want here anyway.
    let mut matches: HashMap<Uuid, RecentMatch> = HashMap::new();
    let expected_total = (query.end - query.start) as usize;
    
    let mut current_start = query.start;
    let mut current_end = query.end;
    let mut existing_video_uuids: HashSet<Uuid> = HashSet::new();
    let mut no_videos_left = false;

    let has_access = app.is_user_allowed_to_es_search(session.user.id).await?;
    while has_access && matches.len() < expected_total {
        let query_size = current_end - current_start;
        // Convert the query and filter into an ElasticSearch query.
        let es_search = filter.to_es_search(session.user.id, Some(&machine_id.id), false)
            .from(current_start)
            .size(current_end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);

        // Get a vector of ESVodDocument which should easily be converted into the RecentMatchPov format (this is a bit of legacy here for having multiple data types).
        let documents: Vec<ESVodDocument> = app.es_api.search_documents(&app.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?;
        let total_documents = documents.len();
        for d in documents {
            if let Some(match_uuid) = d.data.match_uuid {
                if !matches.contains_key(&match_uuid) {
                    matches.insert(match_uuid.clone(), RecentMatch{
                        match_uuid: match_uuid.clone(),
                        game: d.data.game,
                        povs: vec![]
                    });
                }

                let parent_match = matches.get_mut(&match_uuid).unwrap();
                existing_video_uuids.insert(d.vod.video_uuid.clone());

                let new_pov = matches::vod_document_to_match_pov_for_user(d, session.user.id, &machine_id.id);
                parent_match.povs.push(new_pov);
            }
        }

        if total_documents < query_size as usize {
            no_videos_left = true;
            break;
        }

        current_start = current_end;
        current_end += if expected_total >= matches.len() {
            ((expected_total - matches.len()) * 10) as i64
        } else {
            10
        };
    }

    if has_access && !no_videos_left && filter.vods.is_none() {
        // At this point we have found all the matches we want to return to the user - all we need to do now is to find all the remaining VODs that match the query
        // for the matches we've already found. Note that the client will be responsible for stripping out duplicates from future queries.
        filter.matches = Some(matches.keys().cloned().collect());
        filter.not_vods = Some(existing_video_uuids.into_iter().collect());

        let documents: Vec<ESVodDocument> = app.es_api.search_documents(&app.config.elasticsearch.vod_index_read, serde_json::to_value(filter.to_es_search(session.user.id, Some(&machine_id.id), false))?).await?;
        for d in documents {
            if let Some(match_uuid) = d.data.match_uuid {
                // This is an error if I've ever seen one sheeee.
                if !matches.contains_key(&match_uuid) {
                    continue;
                }

                let parent_match = matches.get_mut(&match_uuid).unwrap();
                let new_pov = matches::vod_document_to_match_pov_for_user(d, session.user.id, &machine_id.id);
                parent_match.povs.push(new_pov);
            }
        }
    }

    let mut matches = matches.into_values().collect::<Vec<_>>();
    matches.sort_by(|a, b| {
        b.povs.first().unwrap().tm.partial_cmp(&a.povs.first().unwrap().tm).unwrap()
    });

    // In this case each match needs an access token that can be used to access data for that particular match (VODs, matches, etc.).
    if needs_access_tokens {
        for m in &mut matches {
            for p in &mut m.povs {
                p.access_token = Some(app.generate_access_token_for_recent_match(&m.match_uuid, m.game, p.user_id, &p.vod.video_tracks[0].metadata.video_uuid)?);
            }
        }
    }
    
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response_with_next(&matches, req, &query, if no_videos_left {
        None
    } else {
        Some(api::PaginationParameters{
            start: current_end,
            end: current_end + 20,
        })
    })?)) 
}

pub async fn get_recent_matches_for_me_handler(app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest, query: web::Query<api::PaginationParameters>, filter: web::Json<RecentMatchQuery>, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };

    get_recent_matches_for_user(session.user.id, app, &req, query, filter, false, machine_id).await
}

pub async fn get_profile_matches_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserProfilePath>, req: HttpRequest, query: web::Query<api::PaginationParameters>, mut filter: web::Json<RecentMatchQuery>, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    filter.only_profile = true;
    get_recent_matches_for_user(path.profile_id, app, &req, query, filter, true, machine_id).await
}

#[derive(Deserialize,Debug)]
#[serde(rename_all="camelCase")]
pub struct MatchShareSignatureData {
    full_path: String,
    game: SquadOvGames,
    graphql_stats: Option<Vec<StatPermission>>,
    user_id: i64,
}

pub async fn get_match_share_connections_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    Ok(
        HttpResponse::Ok().json(
            share::get_match_vod_share_connections_for_user(&*app.pool, Some(&path.match_uuid), None, session.user.id).await?
        )
    )
}

pub async fn delete_match_share_link_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    squadov_common::access::delete_encrypted_access_token_for_match_user(&*app.pool, &path.match_uuid, session.user.id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn get_match_share_link_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    let token = squadov_common::access::find_encrypted_access_token_for_match_user(&*app.pool, &path.match_uuid, session.user.id).await?;

    Ok(
        HttpResponse::Ok().json(
            LinkShareData{
                is_link_shared: token.is_some(),
                share_url: if let Some(token) = token {
                    Some(format!(
                        "{}/share/{}",
                        &app.config.cors.domain,
                        &squadov_common::access::get_share_url_identifier_for_id(&*app.pool, &token).await?,
                    ))
                } else {
                    None
                },
            }
        )
    )
}

pub async fn create_match_share_signature_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, data: web::Json<MatchShareSignatureData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    if !squadov_common::matches::is_user_in_match(&*app.pool, session.user.id, &path.match_uuid, data.game).await? {
        let permissions = share::get_match_vod_share_permissions_for_user(&*app.pool, Some(&path.match_uuid), None, session.user.id).await?;
        if !permissions.can_share {
            return Err(SquadOvError::Unauthorized);
        }
    }
    
    // If the user already shared this match, reuse that token so we don't fill up our databases with a bunch of useless tokens.
    let mut token = squadov_common::access::find_encrypted_access_token_for_match_user(&*app.pool, &path.match_uuid, session.user.id).await?;

    // We want to share all the VODs we have access to.
    let vods = vdb::find_accessible_vods_in_match_for_user(&*app.pool, &path.match_uuid.clone(), session.user.id, "").await?;
    
    let mut video_uuids: Vec<Uuid> = vec![];
    for v in vods {
        let can_share = {
            if let Some(user_uuid) = &v.user_uuid {
                user_uuid == &session.user.uuid
            } else {
                false
            }
        } || {
            let permissions = share::get_match_vod_share_permissions_for_user(&*app.pool, None, Some(&v.video_uuid), session.user.id).await?;
            permissions.can_share
        };
        // Sanity check to make sure user has permission to share the VOD itself - otherwise we don't include in the list of VODs the user has access to
        // and don't bother trying to make it public.
        if can_share {
            video_uuids.push(v.video_uuid);
        }
    }

    if token.is_none() {
        // Now that we've verified all these things we can go ahead and return to the user a fully fleshed out
        // URL that can be shared. We enable this by generating an encrypted access token that can be used to imitate 
        // access as this session's user to ONLY this current match UUID (along with an optional VOD UUID if one exists).
        let access_request = AccessTokenRequest{
            full_path: data.full_path.clone(),
            user_uuid: session.user.uuid.clone(),
            meta_user_id: Some(data.user_id),
            match_uuid: Some(path.match_uuid.clone()),
            video_uuid: video_uuids.first().cloned(),
            bulk_video_uuids: video_uuids.clone(),
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
        let token_id = squadov_common::access::store_encrypted_access_token_for_match_user(&mut tx, &path.match_uuid, &video_uuids, session.user.id, data.user_id, &encryption_token).await?;
        squadov_common::access::generate_friendly_share_token(&mut tx, &token_id).await?;
        tx.commit().await?;

        token = Some(token_id);
    }

    // Make the VOD public - we need to keep track of its public setting in our database as well as configure the backend
    // to enable it to be served publically.
    for uuid in &video_uuids {
        app.make_vod_public(&uuid).await?;
    }

    let token = token.ok_or(SquadOvError::InternalError(String::from("Failed to obtain/generate share token.")))?;

    // It could be neat to store some sort of access token ID in our database and allow users to track how
    // many times it was used and be able to revoke it and stuff but I don't think the gains are worth it at
    // the moment. I'd rather have a more distributed version where we toss a URL out there and just let it be
    // valid.
    Ok(
        HttpResponse::Ok().json(
            LinkShareData{
                is_link_shared: true,
                share_url: Some(
                    format!(
                        "{}/share/{}",
                        &app.config.cors.domain,
                        &squadov_common::access::get_share_url_identifier_for_id(&*app.pool, &token).await?,
                    )
                ),
            }
        )
    )
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
    access_token_id: String
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct ShareTokenResponse {
    full_path: String,
    key: String,
    uid: i64,
}

pub async fn exchange_access_token_id_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<ExchangeShareTokenPath>) -> Result<HttpResponse, SquadOvError> {
    let token = squadov_common::access::find_encrypted_access_token_from_flexible_id(&*app.pool, &path.access_token_id).await?;
    let key = token.to_string();
    let req = squadov_decrypt(token, &app.config.squadov.share_key)?;

    let access = serde_json::from_slice::<AccessTokenRequest>(&req.data)?;
    Ok(HttpResponse::Ok().json(&ShareTokenResponse{
        full_path: access.full_path,
        key,
        uid: app.users.get_stored_user_from_uuid(&access.user_uuid, &*app.pool).await?.ok_or(SquadOvError::NotFound)?.id,
    }))
}