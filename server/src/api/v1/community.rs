mod create;
mod get;
mod owner;
mod membership;
mod roles;

pub use create::*;
pub use get::*;
pub use owner::*;
pub use membership::*;
pub use roles::*;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CommunityPathInput {
    pub community_id: i64
}

#[derive(Deserialize)]
pub struct CommunityInvitePathInput {
    pub community_id: i64,
    pub code: Uuid,
}

#[derive(Deserialize)]
pub struct CommunityUserPathInput {
    pub community_id: i64,
    pub user_id: i64,
}

#[derive(Deserialize)]
pub struct CommunityRolePathInput {
    pub community_id: i64,
    pub role_id: i64,
}