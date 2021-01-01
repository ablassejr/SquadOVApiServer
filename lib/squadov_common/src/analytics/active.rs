use crate::SquadOvError;
use sqlx::{Executor, Postgres};

pub async fn mark_active_user_session<'a, T>(ex: &'a mut T, user_id: i64) -> Result<(), SquadOvError>
where
    &'a mut T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.daily_active_sessions (
            user_id,
            tm
        )
        VALUES (
            $1,
            CURRENT_DATE
        )
        ON CONFLICT DO NOTHING
        ",
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn mark_active_user_endpoint<'a, T>(ex: &'a mut T, user_id: i64) -> Result<(), SquadOvError>
where
    &'a mut T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.daily_active_endpoint (
            user_id,
            tm
        )
        VALUES (
            $1,
            CURRENT_DATE
        )
        ON CONFLICT DO NOTHING
        ",
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}