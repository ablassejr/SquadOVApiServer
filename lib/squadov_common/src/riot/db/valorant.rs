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
        &format!(
            "
            SELECT t.id
            FROM (
                VALUES {request}
            ) AS t(id)
            LEFT JOIN squadov.valorant_matches AS vm
                ON vm.match_id = t.id
            WHERE vm.raw_data IS NULL
            ",
            request=match_ids.iter().map(|x| format!("('{}')", x)).collect::<Vec<String>>().join(",")
        )
    )
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
            SELECT vm.match_uuid
            FROM squadov.valorant_match_uuid_link AS vm
            WHERE vm.match_id = $1
            ",
        )
            .bind(match_id)
            .fetch_optional(ex)
            .await?
    )
}