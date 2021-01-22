use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::SquadOvError;
use sqlx::{Executor, Postgres};

#[derive(Serialize)]
pub struct SerializedUserSession {
    #[serde(rename="sessionId")]
    pub session_id: String,
    pub expiration: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct SessionJwtClaims {
    pub exp: i64
}

pub async fn is_valid_temporary_squadov_session<'a, T>(ex: T, session_id: &str) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM squadov.user_sessions
            WHERE id = $1
                AND is_temp = TRUE
        ) AS "valid!"
        "#,
        session_id
    )
        .fetch_one(ex)
        .await?.valid)
}