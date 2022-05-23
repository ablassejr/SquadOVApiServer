use actix_web::{HttpResponse, Result, web};
use serde::{Serialize, Deserialize, Deserializer};
use chrono::{DateTime, Utc};
use std::{
    sync::Arc,
    collections::HashMap,
};
use uuid::Uuid;
use crate::shared::SharedApp;
use elasticsearch_dsl::{Search, Sort, SortOrder, Query};
use squadov_common::{
    SquadOvGames,
    SquadOvWowRelease,
    games,
    elastic::vod::ESVodDocument,
};

#[derive(PartialEq)]
pub enum WowInstanceMode {
    Arena,
    Unknown
}

impl<'de> Deserialize<'de> for WowInstanceMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "arena" => WowInstanceMode::Arena,
            _ => WowInstanceMode::Unknown,
        })
    }
}

#[derive(Clone, PartialEq)]
pub enum WowRelease {
    Retail,
    Tbc,
    Vanilla,
    Unknown
}

impl Into<SquadOvWowRelease> for WowRelease {
    fn into(self) -> SquadOvWowRelease {
        match self {
            WowRelease::Retail => SquadOvWowRelease::Retail,
            WowRelease::Tbc => SquadOvWowRelease::Tbc,
            WowRelease::Vanilla => SquadOvWowRelease::Vanilla,
            _ => SquadOvWowRelease::Retail,
        }
    }
}

impl<'de> Deserialize<'de> for WowRelease {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "retail" => WowRelease::Retail,
            "tbc" => WowRelease::Tbc,
            "vanilla" => WowRelease::Vanilla,
            _ => WowRelease::Unknown,
        })
    }
}

#[derive(PartialEq)]
pub enum WowArenaBracket {
    Skrimish,
    Arena5v5,
    RatedBg,
    Arena3v3,
    Arena2v2,
    Unknown
}

impl<'de> Deserialize<'de> for WowArenaBracket {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "skirmish" => WowArenaBracket::Skrimish,
            "5v5" => WowArenaBracket::Arena5v5,
            "rated bg" => WowArenaBracket::RatedBg,
            "3v3" => WowArenaBracket::Arena3v3,
            "2v2" => WowArenaBracket::Arena2v2,
            _ => WowArenaBracket::Unknown,
        })
    }
}

impl ToString for WowArenaBracket {
    fn to_string(&self) -> String {
        match self {
            WowArenaBracket::Skrimish => "Skirmish",
            WowArenaBracket::Arena5v5 => "5v5",
            WowArenaBracket::RatedBg => "Rated BG",
            WowArenaBracket::Arena3v3 => "3v3",
            WowArenaBracket::Arena2v2 => "2v2",
            WowArenaBracket::Unknown => "Unknown",
        }.to_string()
    }
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct RawWowRequest {
    start_tm: DateTime<Utc>,
    end_tm: DateTime<Utc>,
    mode: WowInstanceMode,
    release: WowRelease,
    patch: Option<String>,
    bracket: Option<WowArenaBracket>,
    page: Option<usize>,
}

#[derive(Serialize)]
pub struct WowCombatantInfo {
    player_guid: String,
    spec_id: i32,
    class_id: Option<i32>,
    rating: i32,
    team: i32,
    items: serde_json::Value,
    talents: serde_json::Value,
    covenant: serde_json::Value,
}

#[derive(Serialize)]
pub struct RawWowResponse {
    id: Uuid,
    tm: chrono::NaiveDateTime,
    build: String,
    // Keep it in json format since we don't really need to process it on the server.
    info: serde_json::Value,
    combatants: Vec<WowCombatantInfo>,
}

const PAGE_SIZE: usize = 1000;

pub async fn raw_wow_handler(payload: web::Json<RawWowRequest>, app: web::Data<Arc<SharedApp>>) -> Result<HttpResponse> {
    if payload.mode == WowInstanceMode::Unknown ||
        payload.release == WowRelease::Unknown
    {
        return Ok(HttpResponse::BadRequest().finish())
    }

    if payload.end_tm.signed_duration_since(payload.start_tm).num_seconds() > 86400 {
        return Ok(HttpResponse::BadRequest().finish())
    }
    
    let mut resp: HashMap<String, RawWowResponse> = HashMap::new();
    let page = payload.page.unwrap_or(0);
    let search_query = Search::new().query({
        let mut q = Query::bool()
            .filter(Query::terms("data.game", vec![SquadOvGames::WorldOfWarcraft as i32]))
            .filter(Query::range("vod.endTime")
                .gte(payload.start_tm.timestamp_millis())
                .lte(payload.end_tm.timestamp_millis())
            )
            .filter(Query::regexp("data.wow.buildVersion", games::wow_release_to_regex_expression(payload.release.clone().into())))
            .filter(Query::term("vod.isClip", false))
        ;

        match payload.mode {
            WowInstanceMode::Arena => {
                q = q.filter(Query::exists("data.wow.arena"));
            }
            _ => (),
        };

        if let Some(patch) = payload.patch.as_ref() {
            q = q.filter(
                Query::regexp("data.wow.buildVersion", patch.as_str())
            );
        }

        if let Some(bracket) = payload.bracket.as_ref() {
            q = q.filter(
                Query::terms("data.wow.arena.type", vec![bracket.to_string()])
            );
        }

        q
    })
        .from(page * PAGE_SIZE)
        .size(PAGE_SIZE)
        .sort(vec![
            Sort::new("vod.endTime")
                .order(SortOrder::Desc)
        ]);
    
    let documents: Vec<ESVodDocument> = app.es_api.search_documents(&app.config.elasticsearch.vod_index_read, serde_json::to_value(search_query)?).await?;
    for d in documents {
        if let Some(match_uuid) = d.data.match_uuid.as_ref() {
            let match_uuid_str = match_uuid.to_string();
            if resp.contains_key(&match_uuid_str) {
                continue;
            }

            if d.vod.end_time.is_none() {
                continue;
            }

            if let Some(wow) = d.data.wow.as_ref() {
                resp.insert(match_uuid_str.clone(), RawWowResponse{
                    id: match_uuid.clone(),
                    tm: d.vod.end_time.map(|x| { x.naive_utc() }).unwrap(),
                    build: wow.build_version.clone(),
                    info: if let Some(arena) = wow.arena.as_ref() {
                        serde_json::to_value(arena)?
                    } else {
                        serde_json::Value::Null
                    },
                    combatants: wow.teams.iter().map(|t| {
                        t.players.iter().map(|p| {
                            WowCombatantInfo {
                                player_guid: p.info.data.guid.clone(),
                                spec_id: p.info.data.spec_id,
                                class_id: p.info.data.class_id.map(|x| { x as i32 }),
                                rating: p.info.data.rating,
                                team: p.info.data.team,
                                items: serde_json::to_value(&p.info.traits.items).unwrap_or(serde_json::Value::Null),
                                talents: serde_json::to_value({
                                    #[derive(Serialize, Clone)]
                                    #[serde(rename_all="camelCase")]
                                    struct TalentWrapper {
                                        talent_id: i32,
                                        is_pvp: bool
                                    }

                                    [
                                        p.info.traits.talents.iter().map(|x| {
                                            TalentWrapper {
                                                talent_id: *x,
                                                is_pvp: false,
                                            }
                                        }).collect::<Vec<_>>().as_slice(),
                                        p.info.traits.pvp_talents.iter().map(|x| {
                                            TalentWrapper {
                                                talent_id: *x,
                                                is_pvp: true,
                                            }
                                        }).collect::<Vec<_>>().as_slice(),
                                    ].concat()
                                }).unwrap_or(serde_json::Value::Null),
                                covenant: serde_json::to_value(&p.info.traits.covenant).unwrap_or(serde_json::Value::Null),
                            }
                        })
                    }).flatten().collect(),
                });
            }
            
        }
    }

    Ok(
        HttpResponse::Ok().json(resp.values().collect::<Vec<_>>())
    )
}