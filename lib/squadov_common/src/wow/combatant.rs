use serde::Deserialize;

#[derive(Deserialize)]
pub struct WoWCombatantInfo {
    pub guid: String
}