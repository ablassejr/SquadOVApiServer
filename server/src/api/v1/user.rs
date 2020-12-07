mod profile;
mod accounts;
mod vod;

pub use profile::*;
pub use accounts::*;
pub use vod::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserResourcePath {
    pub user_id: i64,
}