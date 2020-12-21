use serde::Deserialize;

#[derive(Deserialize)]
pub struct WoWEncounterStart {
    #[serde(rename="encounterId")]
    pub encounter_id: i32,
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    pub difficulty: i32,
    #[serde(rename="numPlayers")]
    pub num_players: i32,
    #[serde(rename="instanceId")]
    pub instance_id: i32
}

#[derive(Deserialize)]
pub struct WoWEncounterEnd {
    #[serde(rename="encounterId")]
    pub encounter_id: i32,
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    pub difficulty: i32,
    #[serde(rename="numPlayers")]
    pub num_players: i32,
    pub success: bool
}

#[derive(Deserialize)]
pub struct WoWChallengeStart {
    #[serde(rename="challengeName")]
    pub challenge_name: String,
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
}

#[derive(Deserialize)]
pub struct WoWChallengeEnd {
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
    pub success: bool,
    #[serde(rename="timeMs")]
    pub time_ms: i64
}