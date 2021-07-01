pub mod db;
pub mod invites;
pub mod roles;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use num_enum::TryFromPrimitive;
use crate::user::SquadOVUser;
use uuid::Uuid;

#[derive(Copy, Clone, Serialize_repr, Deserialize_repr, Debug, TryFromPrimitive, PartialEq)]
#[repr(i32)]
pub enum CommunitySecurityLevel {
    Public,
    Private,
    Unlisted
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct SquadOvCommunity {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub create_tm: DateTime<Utc>,
    pub creator_user_id: i64,
    pub security_level: CommunitySecurityLevel,
    pub requires_subscription: bool,
    pub allow_twitch_sub: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CommunityRole {
    pub id: i64,
    pub community_id: i64,
    pub name: String,
    pub can_manage: bool,
    pub can_moderate: bool,
    pub can_invite: bool,
    pub can_share: bool,
    pub is_default: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all="camelCase")]
pub struct CommunityInvite {
    pub code: Uuid,
    pub community_id: i64,
    pub inviter_user_id: i64,
    pub num_uses: i32,
    pub max_uses: Option<i32>,
    pub expiration: Option<DateTime<Utc>>,
    pub created_tm: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct CommunityUser {
    pub user: SquadOVUser,
    pub sub_id: Option<i64>,
    pub roles: Vec<i64>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct CommunityListQuery {
    #[serde(default)]
    pub only_me: bool,
}