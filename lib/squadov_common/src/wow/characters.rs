use serde::Serialize;

#[derive(Serialize)]
pub struct WoWCharacter {
    pub guid: String,
    pub name: String,
    pub ilvl: i32,
    #[serde(rename="specId")]
    pub spec_id: i32
}