use crate::{
    SquadOvError,
    accounts::TwitchAccount,
    twitch::oauth::TwitchOAuthToken,
};
use sqlx::{Executor, Postgres};

pub async fn update_twitch_account_last_update<'a, T>(ex: T, access_token: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.twitch_accounts
        SET last_validate = NOW()
        WHERE access_token = $1
        ",
        access_token,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn get_twitch_accounts_need_validation<'a, T>(ex: T) -> Result<Vec<TwitchOAuthToken>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TwitchOAuthToken,
            r#"
            SELECT
                access_token AS "access_token!",
                refresh_token AS "refresh_token!",
                '' AS "id_token?",
                (EXTRACT(EPOCH FROM access_expiration) - EXTRACT(EPOCH FROM NOW()))::INTEGER AS "expires_in!"
            FROM squadov.twitch_accounts
            WHERE last_validate < (NOW() - INTERVAL '3 day')
            ORDER BY last_validate ASC
            "#
        )
            .fetch_all(ex)
            .await?
    )
}


pub async fn delete_twitch_account<'a, T>(ex: T, access_token: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.twitch_accounts
        WHERE access_token = $1
        ",
        access_token
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn get_twitch_oauth_token<'a, T>(ex: T, twitch_user_id: &str) -> Result<TwitchOAuthToken, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TwitchOAuthToken,
            r#"
            SELECT
                access_token AS "access_token!",
                refresh_token AS "refresh_token!",
                '' AS "id_token?",
                (EXTRACT(EPOCH FROM access_expiration) - EXTRACT(EPOCH FROM NOW()))::INTEGER AS "expires_in!"
            FROM squadov.twitch_accounts
            WHERE twitch_user_id = $1
            "#,
            twitch_user_id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn find_twitch_account_id<'a, T>(ex: T, twitch_user_id: &str) -> Result<Option<TwitchAccount>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TwitchAccount,
            "
            SELECT ta.twitch_user_id, ta.twitch_name
            FROM squadov.linked_twitch_accounts AS lta
            INNER JOIN squadov.twitch_accounts AS ta
                ON ta.twitch_user_id = lta.twitch_user_id
            WHERE ta.twitch_user_id = $1
            ",
            twitch_user_id,
        )
            .fetch_optional(ex)
            .await?
    )
}

pub async fn find_twitch_accounts_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<TwitchAccount>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TwitchAccount,
            "
            SELECT ta.twitch_user_id, ta.twitch_name
            FROM squadov.linked_twitch_accounts AS lta
            INNER JOIN squadov.twitch_accounts AS ta
                ON ta.twitch_user_id = lta.twitch_user_id
            WHERE lta.user_id = $1
            ",
            user_id
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn update_twitch_access_token<'a, T>(ex: T, old_access_key: &str, token: &TwitchOAuthToken) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.twitch_accounts
        SET access_token = $2,
            refresh_token = $3,
            access_expiration = $4
        WHERE access_token = $1
        ",
        old_access_key,
        token.access_token,
        token.refresh_token,
        token.expiration_time(),
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_twitch_account<'a, T>(ex: T, account: &TwitchAccount, token: &TwitchOAuthToken) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.twitch_accounts (
            twitch_user_id,
            twitch_name,
            access_token,
            refresh_token,
            access_expiration
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        ) ON CONFLICT (twitch_user_id) DO UPDATE SET
            twitch_name = EXCLUDED.twitch_name,
            access_token = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token
        ",
        account.twitch_user_id,
        account.twitch_name,
        token.access_token,
        token.refresh_token,
        token.expiration_time(),
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn link_twitch_account_to_user<'a, T>(ex: T, user_id: i64, twitch_user_id: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.linked_twitch_accounts (
            user_id,
            twitch_user_id,
            linked_tm
        ) VALUES (
            $1,
            $2,
            NOW()
        ) ON CONFLICT DO NOTHING
        ",
        user_id,
        twitch_user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}