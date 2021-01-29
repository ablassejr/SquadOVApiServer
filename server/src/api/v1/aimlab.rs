mod create;
mod list;
mod get_task;

use serde::{Deserialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AimlabTaskGetInput {
    match_uuid: Uuid
}

pub use create::*;
pub use list::*;
pub use get_task::*;