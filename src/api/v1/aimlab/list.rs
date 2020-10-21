use crate::common;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AimlabUserTaskListInput {
    user_id: i64,
}

impl api::ApiApplication {
    pub async fn list_aimlab_matches_for_user(&self, user_id: i64, start: i64, end: i64) -> Result<Vec<super::AimlabTask>, common::SquadOvError> {
        let matches = sqlx::query_as!(
            super::AimlabTask,
            "
            SELECT *
            FROM squadov.aimlab_tasks
            WHERE user_id = $1
            ORDER BY create_date DESC
            LIMIT $2 OFFSET $3
            ",
            user_id,
            end - start,
            start
        )
            .fetch_all(&*self.pool)
            .await?;
        return Ok(matches);
    }
}

pub async fn list_aimlab_matches_for_user_handler(data : web::Path<AimlabUserTaskListInput>, query: web::Query<api::PaginationParameters>, app : web::Data<Arc<api::ApiApplication>>, req: HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let query = query.into_inner();
    let matches = app.list_aimlab_matches_for_user(
        data.user_id,
        query.start,
        query.end,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = matches.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(matches, &req, &query, expected_total == got_total)?)) 
}