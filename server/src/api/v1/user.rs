mod profile;
mod accounts;
mod vod;
mod session;

pub use profile::*;
pub use accounts::*;
pub use vod::*;
pub use session::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserResourcePath {
    pub user_id: i64,
}