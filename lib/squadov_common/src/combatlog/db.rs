use crate::SquadOvError;
use sqlx::{Executor, Postgres};
use chrono::{DateTime, Utc};

pub async fn create_combat_log<'a, T>(ex: T, partition_id: &str, user_id: i64, start_time: DateTime<Utc>, cl_data: serde_json::Value) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        r#"
        INSERT INTO squadov.combat_logs (
            partition_id,
            start_time,
            owner_id,
            cl_state
        ) VALUES (
            $1,
            $2,
            $3,
            $4
        )
        "#,
        partition_id,
        start_time,
        user_id,
        cl_data,
    )
        .execute(ex)
        .await?;

    Ok(())
}

pub async fn get_combat_log_state<'a, T>(ex: T, partition_id: &str) -> Result<serde_json::Value, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT cl_state
            FROM squadov.combat_logs
            WHERE partition_id = $1
            ",
            partition_id,
        )
            .fetch_one(ex)
            .await?
            .cl_state
    )
}