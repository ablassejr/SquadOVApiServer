mod create;
mod delete;
mod edit;

pub use create::*;
pub use delete::*;
pub use edit::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct SquadSelectionInput {
    squad_id: i64
}