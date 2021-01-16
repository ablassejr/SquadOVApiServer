mod create;
mod list;
mod get;

pub use create::*;
pub use list::*;
pub use get::*;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize,Debug)]
pub struct TftMatchInput {
    match_uuid: Uuid
}