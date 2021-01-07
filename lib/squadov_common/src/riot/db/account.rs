use crate::{
    SquadOvError,
    riot::RiotAccount
};
use sqlx::{Executor, Postgres};

pub async fn get_user_riot_account_gamename_tagline<'a, T>(ex: T, user_id: i64, game_name: &str, tag_line: &str) -> Result<RiotAccount, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotAccount,
        "
        SELECT ra.*
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
            AND ra.game_name = $2
            AND ra.tag_line = $3
        ",
        user_id,
        game_name,
        tag_line
    )
        .fetch_one(ex)
        .await?)
}

pub async fn get_user_riot_account<'a, T>(ex: T, user_id: i64, puuid: &str) -> Result<RiotAccount, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotAccount,
        "
        SELECT ra.*
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
            AND ra.puuid = $2
        ",
        user_id,
        puuid
    )
        .fetch_one(ex)
        .await?)
}

pub async fn list_riot_accounts_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<RiotAccount>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotAccount,
        "
        SELECT ra.*
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
        ",
        user_id
    )
        .fetch_all(ex)
        .await?)
}

pub async fn store_riot_account<'a, T>(ex: T, account: &RiotAccount) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.riot_accounts (
            puuid,
            game_name,
            tag_line
        )
        VALUES (
            $1,
            $2,
            $3
        )
        ON CONFLICT (puuid) DO UPDATE
            SET game_name = EXCLUDED.game_name,
                tag_line = EXCLUDED.tag_line
        ",
        account.puuid,
        account.game_name,
        account.puuid
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn link_riot_account_to_user<'a, T>(ex: T, user_id: i64, puuid: &str) -> Result<(), SquadOvError>
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
        ",
        puuid,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}