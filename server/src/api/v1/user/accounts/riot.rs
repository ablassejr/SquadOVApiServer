use actix_web::{web, HttpResponse};
use sqlx::{Executor, Postgres};
use crate::api;
use crate::api::v1::UserResourcePath;
use std::sync::Arc;
use squadov_common::SquadOvError;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LinkRiotAccountInput {
    puuid: String
}

impl api::ApiApplication {
    async fn link_new_riot_account<'a, T>(&self, ex: T, user_id: i64, puuid: &str) -> Result<(), SquadOvError>
    where 
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.riot_account_links (
                puuid,
                user_id
            )
            VALUES (
                $1,
                $2
            )
            ON CONFLICT DO NOTHING
            ",
            puuid,
            user_id
        )
            .execute(ex)
            .await?;
        Ok(())
    }
}

// TODO: When we have access to Riot RSO this needs to handle that instead somehow.
pub async fn link_new_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>, data: web::Json<LinkRiotAccountInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.link_new_riot_account(&mut tx, path.user_id, &data.puuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}