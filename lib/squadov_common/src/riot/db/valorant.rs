mod create;
mod get;
mod list;

use crate::{
    SquadOvError,
};

pub use create::*;
pub use get::*;
pub use list::*;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn get_valorant_matches_that_require_backfill<'a, T>(ex: T, match_ids: &[String]) -> Result<Vec<String>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let matches: Vec<String> = sqlx::query_scalar(
        "
        SELECT t.id
        FROM UNNEST($1) AS t(id)
        LEFT JOIN squadov.valorant_match_uuid_link AS vmul
            ON vmul.match_id = t.id
        WHERE vmul.match_id IS NULL
        "
    )
        .bind(match_ids)
        .fetch_all(ex)
        .await?;
    Ok(matches)
}

pub async fn get_valorant_match_uuid_if_exists<'a, T>(ex: T, match_id: &str) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_scalar(
            "
            SELECT vmul.match_uuid
            FROM squadov.valorant_match_uuid_link AS vmul
            WHERE vmul.match_id = $1
            ",
        )
            .bind(match_id)
            .fetch_optional(ex)
            .await?
    )
}

pub async fn check_valorant_match_details_exist<'a, T>(ex: T, match_id: &str) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_scalar(
            "
            SELECT EXISTS (
                SELECT 1
                FROM squadov.valorant_matches
                WHERE match_id = $1
            )
            "
        )
            .bind(match_id)
            .fetch_one(ex)
            .await?
    )
}