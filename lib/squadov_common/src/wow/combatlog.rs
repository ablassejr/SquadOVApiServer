use serde::Deserialize;

#[derive(Deserialize)]
pub struct WoWCombatLogState {
    #[serde(rename="combatLogVersion")]
    pub combat_log_version: String,
    #[serde(rename="advancedLog")]
    pub advanced_log: bool,
    #[serde(rename="buildVersion")]
    pub build_version: String
}