use serde::{Deserialize};
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct OAuthAccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub expire_time: Option<DateTime<Utc>>
}