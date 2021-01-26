mod profile;
mod accounts;
mod vod;
mod session;
mod notification;
mod status;
mod playtime;

pub use profile::*;
pub use accounts::*;
pub use vod::*;
pub use session::*;
pub use notification::*;
pub use status::*;
pub use playtime::*;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserResourcePath {
    pub user_id: i64,
}