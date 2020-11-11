mod create;
mod list;
mod get;

use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct HearthstoneMatchGetInput {
    match_uuid: Uuid
}

pub use create::*;
pub use list::*;
pub use get::*;