pub mod account;
pub mod valorant;
pub mod shard;
pub mod summoner;
pub mod lol;
pub mod tft;

pub use account::*;
pub use valorant::*;
pub use shard::*;
pub use summoner::*;
pub use lol::*;
pub use tft::*;

use crate::SquadOvError;
use sqlx::{Executor, Postgres};

pub async fn is_riot_puuid_linked_to_user<'a, T>(ex: T, user_id: i64, puuid: &str) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM squadov.riot_account_links AS ral
            WHERE ral.user_id = $1
                AND ral.puuid = $2
        ) AS "exists!"
        "#,
        user_id,
        puuid,
    )
        .fetch_one(ex)
        .await?
        .exists
    )
}
