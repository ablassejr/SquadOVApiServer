use serde::{Deserialize, Serialize};
use crate::{
    SquadOvGames,
    SquadOvError,
    hearthstone::{
        game_packet::HearthstoneGamePacket,
        db as hdb,
    },
    riot::{
        games::{
            lol::{LolParticipantDto, LolTeamDto, LolPlayerMatchSummary},
            tft::{TftPlayerMatchSummary, TftParticipantDto},
            valorant::{ValorantPlayerMatchSummary, ValorantMatchInfoDtoCanonical, ValorantMatchTeamDto, ValorantMatchPlayerDto, ValorantMatchFilterEvents},
        },
        db as rdb,
    },
    wow::{WoWEncounter, WoWChallenge, WoWArena, WowInstance, WowCharacterWrapper},
    aimlab::AimlabTask,
    csgo::summary::CsgoPlayerMatchSummary,
    VodManifest,
    VodAssociation,
    VodMetadata,
    VodTrack,
    vod::{
        db as vdb,
        RawVodTag,
    },
    csgo::{
        db as csgo_db,
    },
    matches::MatchPlayerPair,
    user,
    matches,
    aimlab,
    wow::{
        matches as wm,
        characters as wc,
    },
};
use sqlx::{Executor, Postgres};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESCachedPlayer<PlayerInfo> {
    pub info: PlayerInfo,
    pub is_pov: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESCachedTeam<TeamInfo, PlayerInfo> {
    pub team: TeamInfo,
    pub players: Vec<ESCachedPlayer<PlayerInfo>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodOwner {
    pub user_id: i64,
    pub uuid: Uuid,
    pub username: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodSharing {
    pub squads: Vec<i64>,
    pub users: Vec<i64>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESFavorite {
    pub user_id: i64,
    pub reason: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodParentLists {
    pub watchlist: Vec<i64>,
    pub favorites: Vec<ESFavorite>,
    pub profiles: Vec<i64>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedAimlab {
    pub task: AimlabTask
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedCsgo {
    pub pov: CsgoPlayerMatchSummary,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedHearthstone {
    pub packet: HearthstoneGamePacket
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedLol {
    pub region: String,
    pub summary: Option<LolPlayerMatchSummary>,
    pub teams: Vec<ESCachedTeam<LolTeamDto, LolParticipantDto>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedTft {
    pub region: String,
    pub summary: Option<TftPlayerMatchSummary>,
    pub participants: Vec<ESCachedPlayer<TftParticipantDto>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedValorant {
    pub data: ValorantMatchInfoDtoCanonical,
    pub region: String,
    pub pov_events: Vec<ValorantMatchFilterEvents>,
    pub teams: Vec<ESCachedTeam<ValorantMatchTeamDto, ValorantMatchPlayerDto>>,
    pub summary: Option<ValorantPlayerMatchSummary>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESCachedWowTeam {
    pub id: i32,
    pub won: bool
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedWow {
    pub encounter: Option<WoWEncounter>,
    pub challenge: Option<WoWChallenge>,
    pub arena: Option<WoWArena>,
    pub instance: Option<WowInstance>,
    pub build_version: String,
    pub combat_log_version: String,
    pub advanced_log: bool,
    pub teams: Vec<ESCachedTeam<ESCachedWowTeam, WowCharacterWrapper>>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodCachedMatch {
    pub match_uuid: Option<Uuid>,
    pub game: SquadOvGames,
    pub aimlab: Option<ESVodCachedAimlab>,
    pub csgo: Option<ESVodCachedCsgo>,
    pub hearthstone: Option<ESVodCachedHearthstone>,
    pub lol: Option<ESVodCachedLol>,
    pub tft: Option<ESVodCachedTft>,
    pub valorant: Option<ESVodCachedValorant>,
    pub wow: Option<ESVodCachedWow>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodClip {
    pub title: String,
    pub description: String,
    pub published: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ESVodDocument {
    pub owner: ESVodOwner,
    pub sharing: ESVodSharing,
    pub lists: ESVodParentLists,
    pub data: ESVodCachedMatch,
    pub tags: Vec<RawVodTag>,
    pub manifest: VodManifest,
    pub vod: VodAssociation,
    pub clip: Option<ESVodClip>,
}

impl ESVodDocument {
    pub fn find_favorite_reason(&self, user_id: i64) -> Option<String> {
        self.lists.favorites.iter().find(|x| x.user_id == user_id).map(|x| {
            x.reason.clone()
        })
    }

    pub fn is_on_user_watchlist(&self, user_id: i64) -> bool {
        self.lists.watchlist.iter().any(|x| { *x == user_id })
    }
} 

pub async fn build_es_vod_clip<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Option<ESVodClip>, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    Ok(
        sqlx::query_as!(
            ESVodClip,
            r#"
            SELECT
                title,
                description,
                published
            FROM squadov.vod_clips
            WHERE clip_uuid = $1
            "#,
            video_uuid,
        )
            .fetch_optional(ex)
            .await?
    )
}


pub async fn build_es_vod_document_sharing<'a, T>(ex: T, video_uuid: &Uuid) -> Result<ESVodSharing, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    Ok(
        ESVodSharing{
            squads: vdb::get_vod_shared_to_squads(ex, video_uuid).await?,
            users: vdb::get_vod_shared_to_users(ex, video_uuid).await?,
        }
    )
}

pub async fn build_es_vod_document_lists<'a, T>(ex: T, video_uuid: &Uuid, assoc: &VodAssociation) -> Result<ESVodParentLists, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    Ok(
        ESVodParentLists {
            watchlist: vdb::get_vod_watchlist_ids(ex, video_uuid).await?,
            favorites: if assoc.is_clip {
                vdb::get_vod_favorites(ex, video_uuid).await?
            } else if let Some(match_uuid) = assoc.match_uuid.as_ref() {
                matches::get_match_favorites(ex, match_uuid).await?
            } else {
                vec![]
            }.into_iter().map(|x| {
                ESFavorite{
                    user_id: x.0,
                    reason: x.1,
                }
            }).collect(),
            profiles: vdb::get_vod_profiles(ex, video_uuid).await?,
        }
    )
}

pub async fn build_es_vod_document<'a, T>(ex: T, video_uuid: &Uuid) -> Result<ESVodDocument, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    log::info!("Building ES Vod Document for {}", video_uuid);
    log::info!("...Getting Game {}", video_uuid);
    let game = vdb::get_vod_game(ex, video_uuid).await?;
    log::info!("...Getting VOD {}", video_uuid);
    let assoc = vdb::get_vod_association(ex, video_uuid).await?;
    log::info!("...Getting Manifest {}", video_uuid);
    let manifest = vdb::get_vod_manifest(ex, &assoc).await.unwrap_or(VodManifest{
        video_tracks: vec![
            VodTrack{
                metadata: VodMetadata{
                    video_uuid: video_uuid.clone(),
                    ..VodMetadata::default()
                },
                segments: vec![],
                preview: None,
            }
        ]
    });

    log::info!("...Getting Owner {}", video_uuid);
    let owner = user::get_squadov_user_from_uuid(ex, assoc.user_uuid.as_ref().unwrap()).await?;
    log::info!("...Getting Tags {}", video_uuid);
    let tags = vdb::get_raw_vod_tags(ex, video_uuid).await?;
    log::info!("...Getting Sharing {}", video_uuid);
    let sharing = build_es_vod_document_sharing(ex, video_uuid).await?;
    log::info!("...Getting Lists {}", video_uuid);
    let lists = build_es_vod_document_lists(ex, video_uuid, &assoc).await?;

    let mut data = ESVodCachedMatch{
        match_uuid: assoc.match_uuid.clone(),
        game,
        aimlab: None,
        csgo: None,
        hearthstone: None,
        lol: None,
        tft: None,
        valorant: None,
        wow: None,
    };

    if let Some(match_uuid) = assoc.match_uuid {
        let pair = MatchPlayerPair{
            match_uuid: match_uuid.clone(),
            player_uuid: owner.uuid.clone(),
        };

        log::info!("...Getting Game Data {}", video_uuid);
        match game {
            SquadOvGames::AimLab => {
                data.aimlab = aimlab::list_aimlab_matches_for_uuids(ex, &[match_uuid.clone()]).await?.pop().map(|x| {
                    ESVodCachedAimlab{
                        task: x,
                    }
                });
            },
            SquadOvGames::Csgo => {
                data.csgo = csgo_db::list_csgo_match_summaries_for_uuids(ex, &[pair]).await?.pop().map(|x|{
                    ESVodCachedCsgo{
                        pov: x,
                    }
                });
            },
            SquadOvGames::Hearthstone => {
                data.hearthstone = Some(ESVodCachedHearthstone{
                    packet: hdb::get_hearthstone_game_packet(ex, &match_uuid, owner.id).await?,
                });
            },
            SquadOvGames::LeagueOfLegends => {
                let riot_accounts = rdb::list_riot_summoners_for_user(ex, owner.id).await?;
                data.lol = Some(ESVodCachedLol{
                    region: rdb::get_lol_match_region(ex, &match_uuid).await?,
                    summary: rdb::list_lol_match_summaries_for_uuids(ex, &[pair]).await?.pop(),
                    teams: {
                        let players = rdb::get_lol_match_participants(ex, &match_uuid).await?;
                        let teams = rdb::get_lol_match_teams(ex, &match_uuid).await?;
                        teams.into_iter().map(|t| {
                            ESCachedTeam{
                                players: players.iter().filter(|x| x.team_id == t.team_id).map(|x| {
                                    ESCachedPlayer{
                                        is_pov: riot_accounts.iter().any(|y| { x.puuid == y.puuid || if let Some(summoner_id) = &y.summoner_id {
                                            summoner_id == &x.summoner_id
                                        } else {
                                            false
                                        }}),
                                        info: x.clone(),
                                    }
                                }).collect(),
                                team: t,
                            }
                        }).collect()
                    },
                });
            },
            SquadOvGames::TeamfightTactics => {
                let riot_accounts = rdb::list_riot_accounts_for_user(ex, owner.id).await?;
                data.tft = Some(ESVodCachedTft{
                    region: rdb::get_tft_match_region(ex, &match_uuid).await?,
                    summary: rdb::list_tft_match_summaries_for_uuids(ex, &[pair]).await?.pop(),
                    participants: rdb::get_tft_match_participants(ex, &match_uuid).await?.0.into_iter().map(|x| {
                        ESCachedPlayer{
                            is_pov: riot_accounts.iter().any(|y| { x.puuid == y.puuid }),
                            info: x,
                        }
                    }).collect(),
                });
            },
            SquadOvGames::Valorant => {
                let riot_accounts = rdb::list_riot_accounts_for_user(ex, owner.id).await?;
                data.valorant = Some(ESVodCachedValorant{
                    data: rdb::get_valorant_match_info_dto(ex, &match_uuid).await?.into(),
                    region: rdb::get_valorant_match_shard(ex, &match_uuid).await?,
                    pov_events: rdb::compute_valorant_player_pov_events(ex, &match_uuid, owner.id).await?,
                    teams: {
                        let players = rdb::get_valorant_match_players_dto(ex, &match_uuid).await?;
                        let teams = rdb::get_valorant_match_teams_dto(ex, &match_uuid).await?;
                        teams.into_iter().map(|t| {
                            ESCachedTeam{
                                players: players.iter().filter(|x| x.team_id == t.team_id).map(|x| {
                                    ESCachedPlayer{
                                        is_pov: riot_accounts.iter().any(|y| { x.puuid == y.puuid }),
                                        info: x.clone(),
                                    }
                                }).collect(),
                                team: t,
                            }
                        }).collect()
                    },
                    summary: rdb::list_valorant_match_summaries_for_uuids(ex, &[pair]).await?.pop(),
                });
            },
            SquadOvGames::WorldOfWarcraft => {
                log::info!("...Listing WoW Chars [User] {}", video_uuid);
                let user_chars = wc::list_wow_characters_for_user(ex, owner.id, None).await?;
                log::info!("...Getting Match View {}", video_uuid);
                let match_view = wm::get_generic_wow_match_view_from_match_user(ex, &match_uuid, owner.id).await?;
                log::info!("...List Encounters {}", video_uuid);
                let encounter = wm::list_wow_encounter_for_uuids(ex, &[pair.clone()]).await?.pop();
                log::info!("...List Challenges {}", video_uuid);
                let challenge = wm::list_wow_challenges_for_uuids(ex, &[pair.clone()]).await?.pop();
                log::info!("...List Arenas {}", video_uuid);
                let arena = wm::list_wow_arenas_for_uuids(ex, &[pair.clone()]).await?.pop();
                log::info!("...List Instances {}", video_uuid);
                let instance = wm::list_wow_instances_for_uuids(ex, &[pair.clone()]).await?.pop();

                let force_win = if let Some(e) = &encounter {
                    e.success
                } else if let Some(c) = &challenge {
                    c.success
                } else {
                    false
                };

                let winning_team_id = if let Some(a) = &arena {
                    a.winning_team_id.unwrap_or(-1)
                } else {
                    -1
                };

                let teams = {
                    log::info!("...List WoW Chars [Match] {}", video_uuid);
                    let characters = wc::list_wow_characters_for_match(ex, &match_uuid, owner.id).await?;
                    let mut char_wrappers = vec![];
                    for c in characters {
                        log::info!("...Get Full Char {}", &c.guid);
                        char_wrappers.push(WowCharacterWrapper{
                            traits: wc::get_wow_full_character(ex, &match_view.id, &c.guid).await?,
                            data: c,
                        });
                    }

                    let mut teams: HashMap<i32, ESCachedTeam<ESCachedWowTeam, WowCharacterWrapper>> = HashMap::new();
                    for c in char_wrappers {
                        if !teams.contains_key(&c.data.team) {
                            teams.insert(c.data.team, ESCachedTeam{
                                team: ESCachedWowTeam{
                                    id: c.data.team,
                                    won: force_win || winning_team_id == c.data.team,
                                },
                                players: vec![],
                            });
                        }

                        let team = teams.get_mut(&c.data.team).unwrap();
                        team.players.push(ESCachedPlayer{
                            is_pov: user_chars.iter().any(|x| { x.guid == c.data.guid}),
                            info: c,
                        })
                    }
                    teams.into_values().collect()
                };

                data.wow = Some(ESVodCachedWow{
                    encounter,
                    challenge,
                    arena,
                    instance,
                    build_version: match_view.build_version.clone(),
                    combat_log_version: match_view.combat_log_version.clone(),
                    advanced_log: match_view.advanced_log,
                    teams,
                });
            },
            _ => (),
        }
    }
    
    log::info!("...Finalize and Build Clip {}", video_uuid);
    Ok(
        ESVodDocument{
            owner: ESVodOwner{
                user_id: owner.id,
                uuid: owner.uuid.clone(),
                username: owner.username.clone(),
            },
            sharing,
            lists,
            data,
            tags,
            manifest,
            vod: assoc,
            clip: build_es_vod_clip(ex, video_uuid).await?,
        }
    )
}