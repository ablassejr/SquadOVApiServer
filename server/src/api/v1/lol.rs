mod create;
mod list;
mod get;

pub use create::*;
pub use list::*;
pub use get::*;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize,Debug)]
pub struct LolMatchInput {
    match_uuid: Uuid
}

#[derive(Deserialize)]
pub struct LolMatchUserInput {
    match_uuid: Uuid,
    user_id: i64
}