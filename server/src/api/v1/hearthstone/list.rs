use squadov_common::SquadOvError;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct HearthstoneUserMatchInput {
    user_id: i64,
}

pub struct HearthstoneMatchListEntry {
    match_uuid: Uuid
}

impl api::ApiApplication {
    pub async fn list_hearthstone_matches_for_user(&self, user_id: i64, start: i64, end: i64) -> Result<Vec<Uuid>, squadov_common::SquadOvError> {
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
            ORDER BY hm.id DESC
            LIMIT $2 OFFSET $3
            ",
            user_id,
            end - start,
            start
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|e| { e.match_uuid })
            .collect()
        )
    }
}

pub async fn list_hearthstone_matches_for_user_handler(data : web::Path<HearthstoneUserMatchInput>, query: web::Query<api::PaginationParameters>, app : web::Data<Arc<api::ApiApplication>>, req : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let query = query.into_inner();
    let matches = app.list_hearthstone_matches_for_user(
        data.user_id,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = matches.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(matches, &req, &query, expected_total == got_total)?)) 
}