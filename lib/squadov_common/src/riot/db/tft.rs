mod create;
mod list;
mod get;

pub use create::*;
pub use list::*;
pub use get::*;

use crate::{
    SquadOvError,
    riot::games::TftMatchLink,
};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn get_tft_match_uuid_if_exists<'a, T>(ex: T, platform: &str, game_id: i64) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_scalar(
            "
            SELECT tft.match_uuid
            FROM squadov.tft_matches AS tft
            WHERE tft.platform = $1
                AND tft.match_id = $2
            ",
        )
            .bind(platform)
            .bind(game_id)
            .fetch_optional(ex)
            .await?
    )
}

pub async fn get_tft_match_link_from_uuid<'a, T>(ex: T, match_uuid: &Uuid) -> Result<TftMatchLink, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TftMatchLink,
            "
            SELECT match_uuid, platform, region, match_id
            FROM squadov.tft_matches
            WHERE match_uuid = $1
            ",
            match_uuid,
        )
            .fetch_one(ex)
            .await?
    )
}


pub async fn get_tft_matches_that_require_backfill<'a, T>(ex: T, full_match_ids: &[String]) -> Result<Vec<(String, i64)>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let mut platforms: Vec<String> = Vec::new();
    let mut game_ids: Vec<i64> = Vec::new();

    for mid in full_match_ids {
        let tokens: Vec<&str> = mid.as_str().split("_").collect();
        platforms.push(String::from(tokens[0]));
        game_ids.push(tokens[1].parse()?);
    }

    Ok(
        sqlx::query!(
            r#"
            SELECT t.platform AS "platform!", t.game_id AS "game_id!"
            FROM UNNEST($1::VARCHAR[], $2::BIGINT[]) AS t(platform, game_id)
            LEFT JOIN squadov.tft_matches AS tft
                ON tft.platform = t.platform
                    AND tft.match_id = t.game_id
            WHERE tft.match_id IS NULL
            "#,
            &platforms,
            &game_ids
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                (x.platform, x.game_id)
            })
            .collect()
    )
}