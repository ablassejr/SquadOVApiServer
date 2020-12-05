use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct RiotAccount {
    pub puuid: String,
    pub username: Option<String>,
    pub tag: Option<String>
}