pub mod api;
pub mod db;
pub mod games;
pub mod rso;

use crate::SquadOvError;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use elasticsearch_dsl::{Query, BoolQuery, NestedQuery};

#[derive(Serialize, Deserialize)]
pub struct RiotAccount {
    pub puuid: String,
    #[serde(rename="gameName")]
    pub game_name: Option<String>,
    #[serde(rename="tagLine")]
    pub tag_line: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct RiotSummoner {
    pub puuid: String,
    #[serde(rename="accountId")]
    pub account_id: Option<String>,
    #[serde(rename="summonerId")]
    pub summoner_id: Option<String>,
    #[serde(rename="summonerName")]
    pub summoner_name: Option<String>,
    #[serde(rename="lastBackfillLolTime")]
    pub last_backfill_lol_time: Option<DateTime<Utc>>,
    #[serde(rename="lastBackfillTftTime")]
    pub last_backfill_tft_time: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct RiotSummonerDto {
    #[serde(rename="accountId")]
    pub account_id: String,
    pub name: String,
    pub id: String,
    pub puuid: String
}

#[derive(Deserialize)]
pub struct RiotUserInfo {
    pub sub: String,
    pub cpid: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct LolMatchFilters {
    maps: Option<Vec<i32>>,
    modes: Option<Vec<String>>,
    has_vod: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct TftMatchFilters {
    has_vod: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all="camelCase")]
pub struct ValorantMatchFilters {
    pub maps: Option<Vec<String>>,
    pub modes: Option<Vec<String>>,
    pub has_vod: Option<bool>,
    pub is_ranked: Option<bool>,
    pub agent_povs: Option<Vec<String>>,
    pub is_winner: Option<bool>,
    pub rank_low: Option<i32>,
    pub rank_high: Option<i32>,
    pub pov_events: Option<Vec<games::valorant::ValorantMatchFilterEvents>>,
    pub friendly_composition: Option<Vec<Vec<String>>>,
    pub enemy_composition: Option<Vec<Vec<String>>>,
}

impl ValorantMatchFilters {
    pub fn build_es_query(&self) -> BoolQuery {
        Query::bool()
            .minimum_should_match("1")
            .should(
                Query::bool()
                    .must_not(Query::exists("data.valorant"))
            )
            .should({
                let mut q = Query::bool();

                if let Some(maps) = self.maps.as_ref() {
                    q = q.filter(Query::terms("data.valorant.data.mapId", maps.clone()));
                }

                if let Some(modes) = self.modes.as_ref() {
                    q = q.filter(Query::terms("data.valorant.data.gameMode", modes.clone()));
                }

                if self.is_ranked.unwrap_or(false) {
                    q = q.filter(Query::term("data.valorant.data.isRanked", true));
                }

                {
                    let mut pov_query = Query::bool();
                    pov_query = pov_query.filter(Query::nested(
                        "data.valorant.teams.players",
                        {
                            let mut player_query = Query::bool()
                                .filter(Query::term("data.valorant.teams.players.isPov", true));
                            if let Some(agent_povs) = self.agent_povs.as_ref() {
                                player_query = player_query.filter(
                                    Query::terms(
                                        "data.valorant.teams.players.info.characterId",
                                        agent_povs.iter().map(|x| { x.clone().to_lowercase() }).collect::<Vec<_>>(),
                                    )
                                );
                            }

                            {
                                let mut rank_query = Query::range("data.valorant.teams.players.info.competitiveTier");
                                if let Some(rl) = self.rank_low {
                                    rank_query = rank_query.gte(rl);
                                }
        
                                if let Some(rh) = self.rank_high {
                                    rank_query = rank_query.lte(rh);
                                }
        
                                player_query = player_query.filter(rank_query);
                            }

                            player_query
                        }
                    ));

                    if self.is_winner.unwrap_or(false) {
                        pov_query = pov_query.filter(Query::term("data.valorant.teams.team.won", true));
                    }

                    q = q.filter(Query::nested(
                        "data.valorant.teams",
                        pov_query,
                    ));
                }

                if let Some(pov_events) = self.pov_events.as_ref() {
                    q = q.filter(Query::terms("data.valorant.povEvents", pov_events.iter().map(|x| *x as i32).collect::<Vec<i32>>()));
                }

                {
                    let friendly = self.build_friendly_es_composition_filter();
                    let enemy = self.build_enemy_es_composition_filter();

                    q = q.filter(
                        Query::bool()
                            .minimum_should_match("1")
                            .should(
                                Query::bool()
                                    .filter(
                                        Query::nested(
                                            "data.valorant.teams",
                                            Query::bool()
                                                .filter(Query::term("data.valorant.teams.team.teamId", "Red"))
                                                .filter(friendly.clone())
                                        )
                                    )
                                    .filter(
                                        Query::nested(
                                            "data.valorant.teams",
                                            Query::bool()
                                                .filter(Query::term("data.valorant.teams.team.teamId", "Blue"))
                                                .filter(enemy.clone())
                                        )
                                    )
                            )
                            .should(
                                Query::bool()
                                    .filter(
                                        Query::nested(
                                            "data.valorant.teams",
                                            Query::bool()
                                                .filter(Query::term("data.valorant.teams.team.teamId", "Blue"))
                                                .filter(friendly.clone())
                                        )
                                    )
                                    .filter(
                                        Query::nested(
                                            "data.valorant.teams",
                                            Query::bool()
                                                .filter(Query::term("data.valorant.teams.team.teamId", "Red"))
                                                .filter(enemy.clone())
                                        )
                                    )
                            )
                    );
                }

                q
            })
    }

    pub fn build_friendly_composition_filter(&self) -> Result<String, SquadOvError> {
        ValorantMatchFilters::build_composition_filter(self.friendly_composition.as_ref())
    }

    pub fn build_enemy_composition_filter(&self) -> Result<String, SquadOvError> {
        ValorantMatchFilters::build_composition_filter(self.enemy_composition.as_ref())
    }

    fn build_composition_filter(f: Option<&Vec<Vec<String>>>) -> Result<String, SquadOvError> {
        Ok(
            if let Some(inner) = f {
                let mut pieces: Vec<String> = vec![];
                for x in inner {
                    // It could be empty in which case we want to match anything.
                    if x.is_empty() {
                        continue;
                    }

                    // Each JSON array needs to be converted into a regex lookahead group
                    // that looks like: (?=.*,(1|2|3),)
                    pieces.push(format!(
                        "(?=.*,({}),)",
                        x.into_iter().map(|y| {
                            y.clone().to_lowercase()
                        })
                            .collect::<Vec<String>>()
                            .join("|")
                    ));
                }
                format!("^{}.*$", pieces.join(""))
            } else {
                String::from(".*")
            }
        )
    }

    pub fn build_friendly_es_composition_filter(&self) -> NestedQuery {
        ValorantMatchFilters::build_es_composition_filter(self.friendly_composition.as_ref())
    }

    pub fn build_enemy_es_composition_filter(&self) -> NestedQuery {
        ValorantMatchFilters::build_es_composition_filter(self.enemy_composition.as_ref())
    }

    fn build_es_composition_filter(f: Option<&Vec<Vec<String>>>) -> NestedQuery {
        Query::nested(
            "data.valorant.teams.players",
            {
                let mut q = Query::bool();
                if let Some(inner) = f {
                    let mut has_filter = false;
                    for x in inner {
                        if x.is_empty() {
                            continue;
                        }
                        has_filter = true;
    
                        q = q.should(Query::terms("data.valorant.teams.players.info.characterId", x.iter().map(|y| { y.clone().to_lowercase() }).collect::<Vec<_>>()));
                    }
        
                    if has_filter {
                        q = q.minimum_should_match("1");
                    }
                }
                q
            }
        )
    }
}