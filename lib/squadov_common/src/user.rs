use serde::Serialize;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Clone)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub verified: bool,
    pub uuid: Uuid,
    #[serde(skip_serializing)]
    pub is_test: bool,
    #[serde(skip_serializing)]
    pub is_admin: bool,
    #[serde(skip_serializing)]
    pub welcome_sent: bool,
    #[serde(rename="registrationTime")]
    pub registration_time: Option<DateTime<Utc>>,
}