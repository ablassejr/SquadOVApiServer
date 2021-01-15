use crate::{
    SquadOvError,
    riot::RiotSummoner
};
use sqlx::{Executor, Postgres};

pub async fn get_user_riot_summoner_from_name<'a, T>(ex: T, user_id: i64, summoner_name: &str) -> Result<RiotSummoner, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotSummoner,
        "
        SELECT ra.puuid, ra.account_id, ra.summoner_id, ra.summoner_name, ra.last_backfill_lol_time, ra.last_backfill_tft_time
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
            AND ra.summoner_name = $2
        ",
        user_id,
        summoner_name,
    )
        .fetch_one(ex)
        .await?)
}

pub async fn get_user_riot_summoner<'a, T>(ex: T, user_id: i64, puuid: &str) -> Result<RiotSummoner, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotSummoner,
        "
        SELECT ra.puuid, ra.account_id, ra.summoner_id, ra.summoner_name, ra.last_backfill_lol_time, ra.last_backfill_tft_time
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
            AND ra.puuid = $2
        ",
        user_id,
        puuid,
    )
        .fetch_one(ex)
        .await?)
}

pub async fn list_riot_summoners_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<RiotSummoner>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        RiotSummoner,
        "
        SELECT ra.puuid, ra.account_id, ra.summoner_id, ra.summoner_name, ra.last_backfill_lol_time, ra.last_backfill_tft_time
        FROM squadov.riot_accounts AS ra
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        WHERE ral.user_id = $1
        ",
        user_id,
    )
        .fetch_all(ex)
        .await?)
}

pub async fn store_riot_summoner<'a, T>(ex: T, summoner: &RiotSummoner) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.riot_accounts (
            puuid,
            account_id,
            summoner_id,
            summoner_name
        )
        VALUES (
            $1,
            $2,
            $3,
            $4
        )
        ON CONFLICT (puuid) DO UPDATE
            SET account_id = EXCLUDED.account_id,
                summoner_id = EXCLUDED.summoner_id,
                summoner_name = EXCLUDED.summoner_name
        ",
        summoner.puuid,
        summoner.account_id,
        summoner.summoner_id,
        summoner.summoner_name,
    )
        .execute(ex)
        .await?;
    Ok(())
}