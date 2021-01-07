use crate::{
    SquadOvError,
};
use sqlx::{Executor, Postgres};

pub async fn set_user_account_shard<'a, T>(ex: T, puuid: &str, game: &str, shard: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.riot_account_game_shards (
            puuid,
            game,
            shard
        )
        VALUES (
            $1,
            $2,
            $3
        )
        ON CONFLICT (puuid, game) DO UPDATE
            SET shard = EXCLUDED.shard
        ",
        puuid,
        game,
        shard,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn get_user_account_shard<'a, T>(ex: T, puuid: &str, game: &str) -> Result<String, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_scalar(
        "
        SELECT shard
        FROM squadov.riot_account_game_shards
        WHERE puuid = $1 AND game = $2
        ",
    )
        .bind(puuid)
        .bind(game)
        .fetch_one(ex)
        .await?)
}
