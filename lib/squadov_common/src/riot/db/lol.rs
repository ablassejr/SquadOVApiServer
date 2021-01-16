mod create;
mod list;
mod get;

pub use create::*;
pub use list::*;
pub use get::*;

use crate::{
    SquadOvError,
    riot::games::{
        LolMatchLink,
        LolMatchReferenceDto
    },
};
use sqlx::{Executor, Postgres};
use uuid::Uuid;

pub async fn get_lol_match_uuid_if_exists<'a, T>(ex: T, platform: &str, game_id: i64) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_scalar(
            "
            SELECT lol.match_uuid
            FROM squadov.lol_matches AS lol
            WHERE lol.platform = $1
                AND lol.match_id = $2
            ",
        )
            .bind(platform)
            .bind(game_id)
            .fetch_optional(ex)
            .await?
    )
}

pub async fn get_lol_match_link_from_uuid<'a, T>(ex: T, match_uuid: &Uuid) -> Result<LolMatchLink, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            LolMatchLink,
            "
            SELECT *
            FROM squadov.lol_matches
            WHERE match_uuid = $1
            ",
            match_uuid,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_lol_matches_that_require_backfill<'a, T>(ex: T, match_ids: &[LolMatchReferenceDto]) -> Result<Vec<LolMatchReferenceDto>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let mut platforms: Vec<String> = Vec::new();
    let mut game_ids: Vec<i64> = Vec::new();

    for mi in match_ids {
        platforms.push(mi.platform_id.clone());
        game_ids.push(mi.game_id);
    }

    Ok(
        sqlx::query_as!(
            LolMatchReferenceDto,
            r#"
            SELECT t.platform AS "platform_id!", t.game_id AS "game_id!"
            FROM UNNEST($1::VARCHAR[], $2::BIGINT[]) AS t(platform, game_id)
            LEFT JOIN squadov.lol_matches AS lol
                ON lol.platform = t.platform
                    AND lol.match_id = t.game_id
            WHERE lol.match_id IS NULL
            "#,
            &platforms,
            &game_ids
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn check_lol_match_details_exist<'a, T>(ex: T, platform: &str, game_id: i64) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_scalar(
            "
            SELECT EXISTS (
                SELECT 1
                FROM squadov.lol_match_info
                WHERE platform_id = $1
                    AND game_id = $2
            )
            "
        )
            .bind(platform)
            .bind(game_id)
            .fetch_one(ex)
            .await?
    )
}