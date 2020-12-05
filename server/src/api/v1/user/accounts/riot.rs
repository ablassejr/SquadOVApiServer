use actix_web::{web, HttpResponse};
use sqlx::{Executor, Postgres};
use crate::api;
use crate::api::v1::UserResourcePath;
use serde::Deserialize;
use std::sync::Arc;
use squadov_common::{SquadOvError, RiotAccount};

impl api::ApiApplication {
    async fn link_new_riot_account<'a, T>(&self, ex: T, user_id: i64, input: &RiotAccount) -> Result<(), SquadOvError>
    where 
        T: Executor<'a, Database = Postgres>
    {
        sqlx::query!(
            "
            INSERT INTO squadov.riot_account_links (
                puuid,
                user_id,
                username,
                tag
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ON CONFLICT (user_id, puuid) DO UPDATE
                SET username = EXCLUDED.username,
                    tag = EXCLUDED.tag
            ",
            &input.puuid,
            user_id,
            input.username,
            input.tag
        )
            .execute(ex)
            .await?;
        Ok(())
    }

    async fn list_riot_accounts(&self, user_id: i64) -> Result<Vec<RiotAccount>, SquadOvError> {
        Ok(sqlx::query_as!(
            RiotAccount,
            "
            SELECT puuid, username, tag
            FROM squadov.riot_account_links
            WHERE user_id = $1
            ",
            user_id,
        )
            .fetch_all(&*self.pool)
            .await?)
    }

    pub async fn get_riot_account(&self, user_id: i64, puuid: &str) -> Result<RiotAccount, SquadOvError> {
        Ok(sqlx::query_as!(
            RiotAccount,
            "
            SELECT puuid, username, tag
            FROM squadov.riot_account_links
            WHERE user_id = $1 AND puuid = $2
            ",
            user_id,
            puuid
        )
            .fetch_one(&*self.pool)
            .await?)
    }
}

// TODO: When we have access to Riot RSO this needs to handle that instead somehow.
pub async fn link_new_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>, data: web::Json<RiotAccount>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.link_new_riot_account(&mut tx, path.user_id, &data).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn list_riot_accounts_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let accounts = app.list_riot_accounts(path.user_id).await?;
    Ok(HttpResponse::Ok().json(&accounts))
}

#[derive(Deserialize)]
pub struct RiotAccoutPathInput {
    user_id: i64,
    puuid: String
}

pub async fn get_riot_account_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<RiotAccoutPathInput>) -> Result<HttpResponse, SquadOvError> {
    let account = app.get_riot_account(path.user_id, &path.puuid).await?;
    Ok(HttpResponse::Ok().json(&account))
}