use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Serialize)]
pub struct SerializedUserSession {
    #[serde(rename="sessionId")]
    pub session_id: String,
    pub expiration: DateTime<Utc>
}

#[derive(Deserialize)]
pub struct SessionJwtClaims {
    pub exp: i64
}