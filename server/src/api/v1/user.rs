mod profile;
mod accounts;

pub use profile::*;
pub use accounts::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserResourcePath {
    pub user_id: i64,
}