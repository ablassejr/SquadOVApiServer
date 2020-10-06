use serde::{Deserialize};
use derive_more::{Display, Error};

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
    cfg: SessionConfig
}

#[derive(Debug, Display, Error)]
pub enum SessionError {

}

impl SessionManager {
    pub fn new(cfg : SessionConfig) -> SessionManager {
        return SessionManager{
            cfg: cfg,
        }
    }

    pub fn store_session(&self, session: &SquadOVSession) -> Result<(), SessionError> {
        // Store in database

        return Ok(())
    }
}