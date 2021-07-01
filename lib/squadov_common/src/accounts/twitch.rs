use crate::{
    SquadOvError,
    accounts::TwitchAccount,
    twitch::oauth::TwitchOAuthToken,
};
use sqlx::{Executor, Postgres};

pub async fn find_twitch_accounts_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<TwitchAccount>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            TwitchAccount,
            "
            SELECT twitch_user_id, twitch_name
            FROM squadov.linked_twitch_accounts
            WHERE user_id = $1
            ",
            user_id
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn link_twitch_account_to_user<'a, T>(ex: T, user_id: i64, account: &TwitchAccount, token: &TwitchOAuthToken) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.linked_twitch_accounts (
            user_id,
            twitch_user_id,
            twitch_name,
            access_token,
            refresh_token
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        ) ON CONFLICT (user_id, twitch_user_id) DO UPDATE SET
            twitch_name = EXCLUDED.twitch_name,
            access_token = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token
        ",
        user_id,
        account.twitch_user_id,
        account.twitch_name,
        token.access_token,
        token.refresh_token,
    )
        .execute(ex)
        .await?;
    Ok(())
}