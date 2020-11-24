mod arena;
mod cards;
mod create;
mod deck;
mod list;
mod get;
mod duels;

use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct HearthstoneMatchGetInput {
    match_uuid: Uuid
}

#[derive(Deserialize)]
pub struct HearthstoneCollectionGetInput {
    user_id: i64,
    collection_uuid: Uuid
}

#[derive(Deserialize)]
pub struct HearthstoneUserMatchInput {
    user_id: i64,
}

pub use arena::*;
pub use cards::*;
pub use create::*;
pub use deck::*;
pub use list::*;
pub use get::*;
pub use duels::*;