use serde::Serialize;
use chrono::{DateTime, Utc};
use crate::hearthstone::game_state::{HearthstoneGameBlock, HearthstoneGameSnapshot, HearthstoneGameAction, game_step::GameStep};
use crate::hearthstone::{GameType, FormatType};
use uuid::Uuid;

#[derive(Serialize)]
pub struct HearthstoneSnapshotMetadata {
    pub current_turn: i32,
    pub step: GameStep,
    pub current_player_id: i32,
    pub last_action_id: i64
}

#[derive(Serialize)]
pub struct HearthstoneSerializedSnapshot {
    pub snapshot: HearthstoneGameSnapshot,
    pub metadata: HearthstoneSnapshotMetadata
}

#[derive(Serialize, sqlx::FromRow)]
pub struct HearthstoneMatchMetadata {
    pub game_type: GameType,
    pub format_type: FormatType,
    pub scenario_id: i32,
    pub match_time: DateTime<Utc>
}

#[derive(Serialize)]
pub struct HearthstoneGameLogMetadata {
    pub snapshot_ids: Vec<Uuid>,
    pub num_actions: i64
}

#[derive(Serialize)]
pub struct HearthstoneSerializedGameLog{
    pub snapshots: Vec<HearthstoneSerializedSnapshot>,
    pub actions: Vec<HearthstoneGameAction>,
    pub blocks: Vec<HearthstoneGameBlock>
}

#[derive(Serialize)]
pub struct HearthstoneGamePacket {
    pub match_uuid: Uuid,
    pub metadata: HearthstoneMatchMetadata,
    pub log_metadata: HearthstoneGameLogMetadata,
    pub latest_snapshot: Option<HearthstoneSerializedSnapshot>
}