mod create;

use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct HearthstoneMatchGetInput {
    match_uuid: Uuid
}

pub use create::*;