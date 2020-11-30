use serde::{Deserialize};
use chrono::{DateTime, Utc, Duration};

#[derive(Deserialize)]
pub struct OAuthAccessToken {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i32,
    pub expire_time: Option<DateTime<Utc>>
}

impl OAuthAccessToken {
    pub fn is_expired(&self, buffer_duration: Duration) -> bool {
        if self.expire_time.is_none() {
            true
        } else {
            let expire_time = self.expire_time.unwrap();
            Utc::now() + buffer_duration > expire_time
        }
    }

    pub fn duration_until_expiration(&self, buffer_duration: Duration) -> Result<Duration, crate::SquadOvError> {
        if self.expire_time.is_none() {
            Err(crate::SquadOvError::NotFound)
        } else {
            let target = self.expire_time.unwrap() - buffer_duration;
            let now = Utc::now();
            Ok(if target < now {
                Duration::milliseconds(0)
            } else {
                target - now
            })
        }
    }
}