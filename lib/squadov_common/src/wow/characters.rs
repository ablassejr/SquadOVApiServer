use serde::Serialize;

#[derive(Serialize)]
pub struct WoWCharacter {
    pub guid: String,
    pub name: String,
    pub ilvl: i32,
    #[serde(rename="specId")]
    pub spec_id: i32,
    pub team: i32,
}

#[derive(Serialize)]
pub struct WoWCharacterUserAssociation {
    #[serde(rename="userId")]
    pub user_id: i64,
    pub guid: String
}