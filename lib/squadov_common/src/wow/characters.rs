use serde::Serialize;

#[derive(Serialize)]
pub struct WoWCharacter {
    pub guid: String,
    pub name: String,
    pub ilvl: i32,
    #[serde(rename="specId")]
    pub spec_id: i32,
    pub team: i32,
    pub rating: i32,
}

#[derive(Serialize)]
pub struct WoWCharacterUserAssociation {
    #[serde(rename="userId")]
    pub user_id: i64,
    pub guid: String
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowItem {
    pub item_id: i64,
    pub ilvl: i32,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowCovenant {
    pub covenant_id: i32,
    pub soulbind_id: i32,
    pub soulbind_traits: Vec<i32>,
    pub conduits: Vec<WowItem>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WowFullCharacter {
    pub items: Vec<WowItem>,
    pub covenant: Option<WowCovenant>,
    pub talents: Vec<i32>,
    pub pvp_talents: Vec<i32>,
}