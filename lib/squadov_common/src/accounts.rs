pub mod twitch;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all="camelCase")]
pub struct TwitchAccount {
    pub twitch_user_id: String,
    pub twitch_name: String
}