mod combatlog;
mod matches;

pub use combatlog::*;
pub use matches::*;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WoWMatchPath {
    pub match_uuid: Uuid
}