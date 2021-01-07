pub mod api;
pub mod db;
pub mod games;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct RiotAccount {
    pub puuid: String,
    #[serde(rename="gameName")]
    pub game_name: String,
    #[serde(rename="tagLine")]
    pub tag_line: String
}