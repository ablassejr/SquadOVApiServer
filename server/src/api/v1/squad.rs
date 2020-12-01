mod create;
mod delete;
mod edit;
mod get;

pub use create::*;
pub use delete::*;
pub use edit::*;
pub use get::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SquadSelectionInput {
    squad_id: i64
}