use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use crate::SquadOvError;
use sqlx::{Executor, Postgres};
use async_trait::async_trait;

#[derive(Serialize)]
pub struct SerializedUserSession {
    #[serde(rename="sessionId")]
    pub session_id: String,
    pub expiration: DateTime<Utc>,
    // Number of seconds to expiration
    #[serde(rename="expiresIn")]
    pub expires_in: i64,
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

#[async_trait]
pub trait SessionVerifier {
    async fn verify_session_id_for_user(&self, user_id: i64, session_id: String) -> Result<bool, SquadOvError>;
    async fn verify_user_access_to_users(&self, uid: i64, user_ids: &[i64]) -> Result<bool, SquadOvError>;
}