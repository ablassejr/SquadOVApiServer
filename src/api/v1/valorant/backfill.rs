use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use std::vec::Vec;
use std::sync::Arc;
use sqlx::prelude::Row;

impl api::ApiApplication {
    pub async fn find_nonexistant_valorant_matches(&self, request_matches: &Vec<String>) -> Result<Vec<String>, common::SquadOvError> {
        let matches = sqlx::query(
            &format!(
                "
                SELECT t.id
                FROM (
                    VALUES {request}
                ) AS t(id)
                LEFT JOIN squadov.valorant_matches AS vm
                    ON vm.match_id = t.id
                WHERE vm.match_id IS NULL
                ",
                request=request_matches.iter().map(|x| format!("('{}')", x)).collect::<Vec<String>>().join(",")
            )
        )
            .fetch_all(&*self.pool)
            .await?;

        Ok(matches.iter().map(|x| x.get(0)).collect())
    }
}

pub async fn obtain_valorant_matches_to_backfill(data : web::Json<Vec<String>>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, common::SquadOvError> {
    let ret_matches = app.find_nonexistant_valorant_matches(&data).await?;
    Ok(HttpResponse::Ok().json(&ret_matches))
}