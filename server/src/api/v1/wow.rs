mod combatlog;
mod matches;
mod characters;

pub use combatlog::*;
pub use matches::*;
pub use characters::*;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WoWMatchPath {
    pub match_uuid: Uuid
}

#[derive(Deserialize)]
pub struct WoWUserPath {
    pub user_id: i64
}

#[derive(Deserialize)]
pub struct WoWUserCharacterPath {
    pub user_id: i64,
    pub character_guid: String
}

#[derive(Deserialize)]
pub struct WoWUserMatchPath {
    pub user_id: i64,
    pub match_uuid: Uuid
}