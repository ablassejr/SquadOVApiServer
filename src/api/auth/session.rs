use serde::{Deserialize};
use derive_more::{Display, Error};
use sqlx;
use sqlx::postgres::PgPool;

#[derive(Debug)]
pub struct SquadOVSession {
    pub session_id: String,
    pub user: super::SquadOVUser,
    pub access_token: String,
    pub refresh_token: String
}

#[derive(Deserialize,Debug)]
pub struct SessionConfig {
    encryption_key: String
}

pub struct SessionManager {
    cfg: SessionConfig,
}

#[derive(Debug, Display, Error)]
pub enum SessionError {
    DbError(sqlx::Error)
}

impl SessionManager {
    pub fn new(cfg : SessionConfig) -> SessionManager {
        return SessionManager{
            cfg: cfg,
        }
    }

    pub async fn store_session(&self, session: &SquadOVSession, pool: &PgPool) -> Result<(), SessionError> {
        // Store in database
        match sqlx::query!(
            "
            INSERT INTO squadov.user_sessions (
                id,
                access_token,
                refresh_token,
                user_id
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            session.session_id,
            &session.access_token,
            &session.refresh_token,
            session.user.id
        )
            .execute(pool)
            .await {
            Ok(_) => Ok(()),
            Err(err) => Err(SessionError::DbError(err)),
        }
    }
}