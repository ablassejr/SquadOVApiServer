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
    pub creation_time: DateTime<Utc>,
    #[serde(rename="memberCount")]
    pub member_count: i64,
    #[serde(rename="pendingInviteCount")]
    pub pending_invite_count: i64
}

#[derive(Serialize)]
pub struct SquadOvSquadMembership {
    pub squad: SquadOvSquad,
    pub role: SquadRole,
    pub username: String
}

#[derive(Serialize, sqlx::Type, PartialEq, Debug)]
#[sqlx(rename="squad_role")]
pub enum SquadRole {
    Owner,
    Member
}

#[derive(Serialize)]
pub struct SquadInvite {
    #[serde(rename="squadId")]
    pub squad_id: i64,
    #[serde(rename="userId")]
    pub user_id: i64,
    pub username: String,
    pub joined: bool,
    #[serde(rename="responseTime")]
    pub response_time: Option<DateTime<Utc>>,
    #[serde(rename="inviteTime")]
    pub invite_time: Option<DateTime<Utc>>,
    #[serde(rename="inviteUuid")]
    pub invite_uuid: uuid::Uuid,
    #[serde(rename="inviterUsername")]
    pub inviter_username: String
}