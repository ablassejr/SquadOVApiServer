pub mod power_parser;
pub mod game_state;
pub mod game_packet;
pub mod db;

mod game_type;
mod format_type;

pub use game_type::*;
pub use format_type::*;

use crate::SquadOvError;
use serde::{Serialize,Deserialize};
use chrono::{DateTime, Utc, serde::ts_seconds};
use ipnetwork::IpNetwork;
use uuid::Uuid;
use std::collections::HashMap;
use sqlx::{Executor, Postgres};

#[derive(Deserialize)]
pub struct HearthstoneGameConnectionInfo {
    pub ip: IpNetwork,
    pub port: i32,
    #[serde(rename = "gameId")]
    pub game_id: i32,
    #[serde(rename = "clientId")]
    pub client_id: i32,
    #[serde(rename = "spectateKey")]
    pub spectate_key: String,
    pub reconnecting: bool
}

#[derive(Deserialize,Serialize,PartialEq)]
pub struct HearthstoneCardCount {
    pub normal: i32,
    pub golden: i32
}

#[derive(Deserialize,Serialize,PartialEq)]
pub struct HearthstoneDeckSlot {
    pub index: i32,
    #[serde(rename = "cardId")]
    pub card_id: String,
    pub owned: bool,
    pub count: HearthstoneCardCount
}

pub fn are_deck_slots_equivalent(a: &[HearthstoneDeckSlot], b: &[HearthstoneDeckSlot]) -> bool {
    let mut a_map: HashMap<String, &HearthstoneDeckSlot> = HashMap::new();
    for a_slot in a {
        a_map.insert(a_slot.card_id.clone(), a_slot);
    }

    let mut b_map: HashMap<String, &HearthstoneDeckSlot> = HashMap::new();
    for b_slot in b {
        b_map.insert(b_slot.card_id.clone(), b_slot);
    }

    for b_slot in b {
        if !a_map.contains_key(&b_slot.card_id) {
            return false;
        }

        if a_map.get(&b_slot.card_id).unwrap() != &b_slot {
            return false;
        }
    }

    for a_slot in a {
        if !b_map.contains_key(&a_slot.card_id) {
            return false;
        }
    }

    true
}

#[derive(Serialize)]
pub struct HearthstoneCardMetadata {
    #[serde(rename = "cardId")]
    pub card_id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: i32
}

#[derive(Serialize)]
pub struct HearthstoneBattlegroundsCardMetadata {
    pub base: HearthstoneCardMetadata,
    #[serde(rename = "tavernLevel")]
    pub tavern_level: i32,
    #[serde(rename = "cardRace")]
    pub card_race: Option<i32>
}

#[derive(Deserialize,Serialize)]
pub struct HearthstoneDeck {
    pub name: String,
    #[serde(rename = "deckId")]
    pub deck_id: i64,
    #[serde(rename = "heroCard")]
    pub hero_card: String,
    #[serde(rename = "heroPremium")]
    pub hero_premium: i32,
    #[serde(rename = "deckType")]
    pub deck_type: i32,
    #[serde(rename = "createDate", with="ts_seconds")]
    pub create_date: DateTime<Utc>,
    #[serde(rename = "isWild")]
    pub is_wild: bool,
    pub slots: Vec<HearthstoneDeckSlot>
}

#[derive(Deserialize,Serialize)]
pub struct HearthstoneMedalInfo {
    #[serde(rename = "leagueId")]
    pub league_id: i32,
    #[serde(rename = "earnedStars")]
    pub earned_stars: i32,
    #[serde(rename = "starLevel")]
    pub star_level: i32,
    #[serde(rename = "bestStarLevel")]
    pub best_star_level: i32,
    #[serde(rename = "winStreak")]
    pub win_streak: i32,
    #[serde(rename = "legendIndex")]
    pub legend_index: i32
}

impl Default for HearthstoneMedalInfo {
    fn default() -> Self {
        Self {
            league_id: 0,
            earned_stars: 0,
            star_level: 0,
            best_star_level: 0,
            win_streak: 0,
            legend_index: 0
        }
    }
}

#[derive(Deserialize,Serialize)]
pub struct HearthstonePlayerMedalInfo {
    pub standard: HearthstoneMedalInfo,
    pub wild: HearthstoneMedalInfo
}

impl HearthstonePlayerMedalInfo {
    pub fn new() -> Self {
        Self {
            standard: HearthstoneMedalInfo{
                ..Default::default()
            },
            wild: HearthstoneMedalInfo{
                ..Default::default()
            }
        }
    }
}

#[derive(Deserialize,Serialize)]
pub struct HearthstonePlayer {
    pub name: String,
    pub local: bool,
    pub side: i32,
    #[serde(rename = "cardBackId")]
    pub card_back_id: i32,
    #[serde(rename = "medalInfo")]
    pub medal_info: HearthstonePlayerMedalInfo,
    #[serde(rename = "arenaWins")]
    pub arena_wins: u32,
    #[serde(rename = "arenaLoss")]
    pub arena_loss: u32,
    #[serde(rename = "tavernBrawlWins")]
    pub tavern_brawl_wins: u32,
    #[serde(rename = "tavernBrawlLoss")]
    pub tavern_brawl_loss: u32,
    #[serde(rename = "battlegroundsRating")]
    pub battlegrounds_rating: Option<i32>,
    #[serde(rename = "duelsCasualRating")]
    pub duels_casual_rating: Option<i32>,
    #[serde(rename = "duelsHeroicRating")]
    pub duels_heroic_rating: Option<i32>
}

#[derive(Serialize,Deserialize)]
pub struct HearthstoneRawLog {
    pub time: DateTime<Utc>,
    pub section: String,
    pub log: String
}

#[derive(Serialize)]
pub struct HearthstoneArenaRun {
    #[serde(rename = "arenaUuid")]
    pub arena_uuid: Uuid,
    pub deck: Option<HearthstoneDeck>,
    pub wins: u32,
    pub loss: u32,
    pub timestamp: DateTime<Utc>
}

#[derive(Serialize)]
pub struct HearthstoneDuelRun {
    #[serde(rename = "duelUuid")]
    pub duel_uuid: Uuid,
    #[serde(rename = "heroCard")]
    pub hero_card: Option<String>,
    pub deck: Option<HearthstoneDeck>,
    pub wins: i64,
    pub loss: i64,
    pub rating: Option<i32>,
    pub timestamp: DateTime<Utc>
}

pub async fn is_user_in_hearthstone_match<'a, T>(ex: T, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM squadov.hearthstone_match_view
                WHERE match_uuid = $1
                    AND user_id = $2
            ) AS "exists!"
            "#,
            match_uuid,
            user_id,
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}