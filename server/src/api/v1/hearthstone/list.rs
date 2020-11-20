use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use std::sync::Arc;
use uuid::Uuid;
use squadov_common::hearthstone::{GameType, get_all_hearthstone_game_types};
use serde::{Deserialize};
use std::convert::TryFrom;

pub struct HearthstoneMatchListEntry {
    match_uuid: Uuid
}

#[derive(Deserialize)]
pub struct FilteredMatchParameters {
    pub start: i64,
    pub end: i64,
    pub filter: String
}

impl api::ApiApplication {
    pub async fn list_hearthstone_matches_for_user(&self, user_id: i64, start: i64, end: i64, filters: &[GameType]) -> Result<Vec<Uuid>, squadov_common::SquadOvError> {
        // Need to inner join on hearthstone_match_metadata as we won't be able to
        // successfully display the match otherwise.
        Ok(sqlx::query_as!(
            HearthstoneMatchListEntry,
            "
            SELECT hm.match_uuid
            FROM squadov.hearthstone_matches AS hm
            INNER JOIN squadov.hearthstone_match_players AS hmp
                ON hmp.match_uuid = hm.match_uuid
            INNER JOIN squadov.hearthstone_match_metadata AS hmm
                ON hmm.match_uuid = hm.match_uuid
            WHERE hmp.user_id = $1
                AND hmm.game_type = any($4)
            ORDER BY hm.id DESC
            LIMIT $2 OFFSET $3
            ",
            user_id,
            end - start,
            start,
            &filters.iter().map(|e| { e.clone() as i32 }).collect::<Vec<i32>>()
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|e| { e.match_uuid })
            .collect()
        )
    }
}

pub async fn list_hearthstone_matches_for_user_handler(data : web::Path<super::HearthstoneUserMatchInput>, query: web::Query<FilteredMatchParameters>, app : web::Data<Arc<api::ApiApplication>>, req : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let gametype_filter = if query.filter.is_empty() {
        vec![]
    } else {
        serde_json::from_str::<Vec<i32>>(&query.filter)?.into_iter().map(|e| { GameType::try_from(e).unwrap_or(GameType::Unknown) }).collect::<Vec<GameType>>()
    };

    let matches = app.list_hearthstone_matches_for_user(
        data.user_id,
        query.start,
        query.end,
        if gametype_filter.is_empty() {
            get_all_hearthstone_game_types()
        } else {
            &gametype_filter
        },
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = matches.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(matches, &req, &api::PaginationParameters{
        start: query.start,
        end: query.end,
    }, expected_total == got_total)?)) 
}