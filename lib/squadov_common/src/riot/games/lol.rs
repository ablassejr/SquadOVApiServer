use uuid::Uuid;
use serde::Deserialize;

pub struct LolMatchLink {
    pub match_uuid: Uuid,
    pub platform: String,
    pub match_id: i64
}

#[derive(Deserialize)]
pub struct LolMatchlistDto {
    pub matches: Vec<LolMatchReferenceDto>
}

#[derive(Deserialize)]
pub struct LolMatchReferenceDto {
    #[serde(rename="gameId")]
    pub game_id: i64,
    #[serde(rename="platformId")]
    pub platform_id: String
}