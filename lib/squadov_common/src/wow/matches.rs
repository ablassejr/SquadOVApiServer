use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

#[derive(Serialize, Deserialize, Clone)]
pub struct WoWEncounter {
    #[serde(rename="matchUuid")]
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    #[serde(rename="combatantsKey")]
    pub combatants_key: String,
    #[serde(rename="encounterId")]
    pub encounter_id: i32,
    #[serde(rename="encounterName")]
    pub encounter_name: String,
    pub difficulty: i32,
    #[serde(rename="numPlayers")]
    pub num_players: i32,
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="finishTime")]
    pub finish_time: Option<DateTime<Utc>>,
    pub success: bool,
    pub build: String
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

#[derive(Serialize, Deserialize, Clone)]
pub struct WoWChallenge {
    #[serde(rename="matchUuid")]
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    #[serde(rename="combatantsKey")]
    pub combatants_key: String,
    #[serde(rename="challengeName")]
    pub challenge_name: String,
    #[serde(rename="instanceId")]
    pub instance_id: i32,
    #[serde(rename="keystoneLevel")]
    pub keystone_level: i32,
    #[serde(rename="finishTime")]
    pub finish_time: Option<DateTime<Utc>>,
    #[serde(rename="timeMs")]
    pub time_ms: i64,
    pub success: bool,
    pub build: String
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoWArenaStart {
    pub instance_id: i32,
    #[serde(rename="type")]
    pub arena_type: String,
    pub local_team_id: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WoWArenaEnd {
    pub winning_team_id: i32,
    pub match_duration_seconds: i32,
    pub new_ratings: Vec<i32>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WoWArena {
    pub match_uuid: Uuid,
    pub tm: DateTime<Utc>,
    pub combatants_key: String,
    pub instance_id: i32,
    #[serde(rename="type")]
    pub arena_type: String,
    pub finish_time: Option<DateTime<Utc>>,
    pub winning_team_id: Option<i32>,
    pub match_duration_seconds: Option<i32>,
    pub new_ratings: Option<Vec<i32>>,
    pub user_uuid: Uuid,
    pub success: bool,
    pub build: String
}