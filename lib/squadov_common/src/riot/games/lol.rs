use uuid::Uuid;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

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

#[derive(Serialize,Deserialize)]
pub struct LolMatchDto {
    #[serde(rename="gameId")]
    pub game_id: i64,
    #[serde(rename="queueId")]
    pub queue_id: i32,
    #[serde(rename="gameType")]
    pub game_type: String,
    #[serde(rename="gameDuration")]
    pub game_duration: i64, // seconds
    #[serde(rename="platformId")]
    pub platform_id: String,
    #[serde(rename="gameCreation", deserialize_with="crate::parse_utc_time_from_milliseconds")]
    pub game_creation: Option<DateTime<Utc>>, // timestamp when champ select ended and loading screen appears
    #[serde(rename="seasonId")]
    pub season_id: i32,
    #[serde(rename="gameVersion")]
    pub game_version: String,
    #[serde(rename="mapId")]
    pub map_id: i32,
    #[serde(rename="gameMode")]
    pub game_mode: String,
    #[serde(rename="participantIdentities")]
    pub participant_identities: Vec<LolParticipantIdentityDto>,
    pub teams: Vec<LolTeamStatsDto>,
    pub participants: Vec<LolParticipantDto>,
}

#[derive(Serialize,Deserialize)]
pub struct LolParticipantIdentityDto {
    #[serde(rename="participantId")]
    pub participant_id: i32,
    pub player: Option<LolPlayerDto>
}

#[derive(Serialize,Deserialize)]
pub struct LolPlayerDto {
    #[serde(rename="accountId")]
    pub account_id: String,
    #[serde(rename="currentAccountId")]
    pub current_account_id: String,
    #[serde(rename="currentPlatformId")]
    pub current_platform_id: String,
    #[serde(rename="summonerName")]
    pub summoner_name: String,
    #[serde(rename="summonerId")]
    pub summoner_id: Option<String>,
    #[serde(rename="platformId")]
    pub platform_id: String,
}

#[derive(Serialize,Deserialize)]
pub struct LolTeamStatsDto {
    #[serde(rename="towerKills")]
    pub tower_kills: i32,
    #[serde(rename="riftHeraldKills")]
    pub rift_herald_kills: i32,
    #[serde(rename="firstBlood")]
    pub first_blood: bool,
    #[serde(rename="inhibitorKills")]
    pub inhibitor_kills: i32,
    #[serde(rename="firstBaron")]
    pub first_baron: bool,
    #[serde(rename="firstDragon")]
    pub first_dragon: bool,
    #[serde(rename="dragonKills")]
    pub dragon_kills: i32,
    #[serde(rename="baronKills")]
    pub baron_kills: i32,
    #[serde(rename="firstInhibitor")]
    pub first_inhibitor: bool,
    #[serde(rename="firstTower")]
    pub first_tower: bool,
    #[serde(rename="firstRiftHerald")]
    pub first_rift_herald: bool,
    #[serde(rename="teamId")]
    pub team_id: i32,
    pub win: String,
    pub bans: Vec<LolTeamBansDto>
}

#[derive(Serialize,Deserialize)]
pub struct LolTeamBansDto {
    #[serde(rename="championId")]
    pub champion_id: i32,
    #[serde(rename="pickTurn")]
    pub pick_turn: i32
}

#[derive(Serialize,Deserialize)]
pub struct LolParticipantDto {
    #[serde(rename="participantId")]
    pub participant_id: i32,
    #[serde(rename="championId")]
    pub champion_id: i32,
    #[serde(rename="teamId")]
    pub team_id: i32,
    #[serde(rename="spell1Id")]
    pub spell1_id: i32,
    #[serde(rename="spell2Id")]
    pub spell2_id: i32,
    pub stats: LolParticipantStatsDto,
    pub timeline: LolParticipantTimelineDto
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolParticipantTimelineDto {
    pub participant_id: i32,
    pub lane: String
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolParticipantStatsDto {
    // Player Identity
    pub participant_id: i32,
    #[serde(default)]
    pub champ_level: i32,
    #[serde(default)]
    pub win: bool,
    // KDA
    #[serde(default)]
    pub kills: i32,
    #[serde(default)]
    pub deaths: i32,
    #[serde(default)]
    pub assists: i32,
    // Items
    #[serde(default)]
    pub item0: i32,
    #[serde(default)]
    pub item1: i32,
    #[serde(default)]
    pub item2: i32,
    #[serde(default)]
    pub item3: i32,
    #[serde(default)]
    pub item4: i32,
    #[serde(default)]
    pub item5: i32,
    #[serde(default)]
    pub item6: i32,
    // Notable kills
    #[serde(default)]
    pub double_kills: i32,
    #[serde(default)]
    pub triple_kills: i32,
    #[serde(default)]
    pub quadra_kills: i32,
    #[serde(default)]
    pub penta_kills: i32,
    #[serde(default)]
    pub first_blood_kill: bool,
    // Econ Stats
    #[serde(default)]
    pub gold_earned: i32,
    #[serde(default)]
    pub gold_spent: i32,
    // Neutral Stats
    #[serde(default)]
    pub neutral_minions_killed_team_jungle: i32,
    #[serde(default)]
    pub neutral_minions_killed_enemy_jungle: i32,
    #[serde(default)]
    pub wards_killed: i32,
    #[serde(default)]
    pub wards_placed: i32,
    #[serde(default)]
    pub vision_wards_bought_in_game: i32,
    #[serde(default)]
    pub sight_wards_bought_in_game: i32,
    #[serde(default)]
    pub neutral_minions_kills: i32,
    #[serde(default)]
    pub total_minions_killed: i32,
    // Objective Stats
    #[serde(default)]
    pub damage_dealt_to_objectives: i64,
    #[serde(default)]
    pub inhibitor_kills: i32,
    #[serde(default)]
    pub turret_kills: i32,
    #[serde(default)]
    pub damage_dealt_to_turrets: i64,
    // Score
    #[serde(default)]
    pub total_player_score: i32,
    #[serde(default)]
    pub total_score_rank: i32,
    #[serde(default)]
    pub objective_player_score: i32,
    #[serde(default)]
    pub combat_player_score: i32,
    #[serde(default)]
    pub vision_score: i64,
    // Damage Dealt to Champions
    #[serde(default)]
    pub total_damage_dealt_to_champions: i64,
    #[serde(default)]
    pub physical_damage_dealt_to_champions: i64,
    #[serde(default)]
    pub magic_damage_dealt_to_champions: i64,
    #[serde(default)]
    pub true_damage_dealt_to_champions: i64,
    // Damage Dealt
    #[serde(default)]
    pub total_damage_dealt: i64,
    #[serde(default)]
    pub physical_damage_dealt: i64,
    #[serde(default)]
    pub magic_damage_dealt: i64, 
    #[serde(default)]
    pub true_damage_dealt: i64,
    // Damage Taken
    #[serde(default)]
    pub total_damage_taken: i64, 
    #[serde(default)]
    pub physical_damage_taken: i64,
    #[serde(default)]
    pub magical_damage_taken: i64,
    #[serde(default)]
    pub true_damage_taken: i64,
    // Other Combat  
    #[serde(default)]  
    pub total_heal: i64,
    #[serde(default)]
    pub damage_self_mitigated: i64,
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolMatchTimelineDto {
    pub frames: Vec<LolMatchFrameDto>,
    pub frame_interval: i64
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolMatchFrameDto {
    pub participant_frames: HashMap<String, LolMatchParticipantFrameDto>,
    pub events: Vec<LolMatchEventDto>,
    pub timestamp: i64,
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolMatchParticipantFrameDto {
    pub participant_id: i32,
    pub minions_killed: i32,
    pub total_gold: i32,
    pub level: i32,
    pub xp: i32,
    pub current_gold: i32,
    pub jungle_minions_killed: i32,
    pub position: Option<LolMatchPositionDto>,
}

#[derive(Serialize,Deserialize)]
pub struct LolMatchPositionDto {
    pub x: i32,
    pub y: i32,
}

#[derive(Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LolMatchEventDto {
    pub lane_type: Option<String>,
    pub skill_slot: Option<i32>,
    pub ascended_type: Option<String>,
    pub creator_id: Option<i32>,
    pub after_id: Option<i32>,
    pub event_type: Option<String>,
    #[serde(rename="type")]
    pub real_type: String,
    pub level_up_type: Option<String>,
    pub ward_type: Option<String>,
    pub participant_id: Option<i32>,
    pub tower_type: Option<String>,
    pub item_id: Option<i32>,
    pub before_id: Option<i32>,
    pub monster_type: Option<String>,
    pub monster_sub_type: Option<String>,
    pub team_id: Option<i32>,
    pub position: Option<LolMatchPositionDto>,
    pub killer_id: Option<i32>,
    pub timestamp: i64,
    pub assisting_participant_ids: Option<Vec<i32>>,
    pub building_type: Option<String>,
    pub victim_id: Option<i32>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LolMiniParticipantStats {
    pub participant_id: i32,
    pub champion_id: i32,
    pub summoner_name: Option<String>,
    pub team_id: i32,
    pub kills: i32,
    pub deaths: i32,
    pub assists: i32,
    pub total_damage_dealt_to_champions: i64,
    pub total_minions_killed: i32,
    pub wards_placed: i32,
    pub lane: String,
    pub win: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LolPlayerMatchSummary {
    pub match_uuid: Uuid,
    pub user_uuid: Uuid,
    pub game_creation: DateTime<Utc>,
    pub game_duration: i64,
    pub game_type: String,
    pub game_version: String,
    pub queue_id: i32,
    pub season_id: i32,
    pub map_id: i32,
    pub game_mode: String,
    pub current_participant_id: i32,
    pub participants: Vec<LolMiniParticipantStats>,
    pub has_vod: bool,
}

impl LolPlayerMatchSummary {
    fn did_win(&self) -> bool {
        for x in &self.participants {
            if x.participant_id == self.current_participant_id {
                return x.win;
            }
        }
        false
    }

    pub fn win_loss(&self) -> String {
        if self.did_win() {
            String::from("Win")
        } else {
            String::from("Loss")
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullLolMatch {
    #[serde(rename="match")]
    pub lol_match: LolMatchDto,
    pub timeline: LolMatchTimelineDto,
    pub user_id_to_participant_id: HashMap<i64, i32>,
    pub game_start_time: Option<DateTime<Utc>>,
}