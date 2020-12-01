use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Serialize)]
pub struct SquadOvSquad {
    pub id: i64,
    #[serde(rename="squadName")]
    pub squad_name: String,
    #[serde(rename="squadGroup")]
    pub squad_group: String,
    #[serde(rename="creationTime")]
    pub creation_time: DateTime<Utc>
}

#[derive(sqlx::Type, PartialEq)]
#[sqlx(rename="squad_role")]
pub enum SquadRole {
    Owner,
    Member
}