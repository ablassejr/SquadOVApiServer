use serde::Serialize;

#[derive(Serialize)]
pub struct SteamAccount {
    pub steam_id: i64,
    pub name: String
}