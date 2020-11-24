pub mod game_entity;
pub mod game_step;
pub mod player_entity;
pub mod play_state;

use derive_more::{Display};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use regex::Regex;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};
use num_enum::TryFromPrimitive;
use std::str::FromStr;

#[derive(Display, Clone)]
pub enum EntityId {
    GameEntity,
    #[display(fmt = "NewGameEntitty {}", _0)]
    NewGameEntity(i32),
    #[display(fmt = "Player {}", _0)]
    Player(String),
    #[display(fmt = "NewPlayer E:{} P:{}", entity_id, player_id)]
    NewPlayer{
        entity_id: i32,
        player_id: i32
    },
    #[display(fmt = "New {}", _0)]
    New(i32),
    #[display(fmt = "Existing {}", _0)]
    Existing(String),
    #[display(fmt = "None")]
    None
}

impl std::default::Default for EntityId {
    fn default() -> Self {
        EntityId::None
    }
}

#[derive(sqlx::Type, Display, Clone, Copy, Serialize_repr)]
#[repr(i32)]
pub enum BlockType {
    Invalid = 0,
    Attack = 1,
    Joust = 2,
    Power = 3,
    Trigger = 5,
    Deaths = 6,
    Play = 7,
    Fatigue = 8,
    Ritual = 9,
    RevealCard = 10,
    GameReset = 11,
    MoveMinion = 12
}

#[derive(sqlx::Type, Display, Clone, Copy, Serialize_repr, Deserialize_repr, TryFromPrimitive)]
#[repr(i32)]
pub enum ActionType {
    CreateGame,
    CreatePlayer,
    FullEntity,
    ShowEntity,
    TagChange
}

impl std::str::FromStr for BlockType {
    type Err = crate::SquadOvError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "INVALID" => BlockType::Invalid,
            "ATTACK" => BlockType::Attack,
            "JOUST" => BlockType::Joust,
            "POWER" => BlockType::Power,
            "TRIGGER" => BlockType::Trigger,
            "DEATHS" => BlockType::Deaths,
            "PLAY" => BlockType::Play,
            "FATIGUE" => BlockType::Fatigue,
            "RITUAL" => BlockType::Ritual,
            "REVEAL_CARD" => BlockType::RevealCard,
            "GAME_RESET" => BlockType::GameReset,
            "MOVE_MINION" => BlockType::MoveMinion,
            _ => return Err(crate::SquadOvError::NotFound)
        })
    }
}

// A block represents a group of actions that represent one "logical" action.
// A "logical" action is, for example, something that the player does (e.g. playing a card).
// While the user only did one thing, the game has to perform actions to accomplish that.
#[derive(sqlx::FromRow, Display, Clone, Serialize)]
#[display(fmt="HearthstoneGameBlock[Uuid: {} Parent: {:?} Start: {} End: {} Type: {} Entity: {}]", block_id, parent_block, start_action_index, end_action_index, block_type, entity_id)]
pub struct HearthstoneGameBlock {
    #[serde(rename = "blockId")]
    pub block_id: Uuid,
    #[serde(rename = "startActionIndex")]
    pub start_action_index: i32,
    #[serde(rename = "endActionIndex")]
    pub end_action_index: i32,
    #[serde(rename = "blockType")]
    pub block_type: BlockType,
    #[serde(rename = "parentBlock")]
    pub parent_block: Option<Uuid>,
    #[serde(rename = "entityId")]
    pub entity_id: i32
}

// Generally actions are just a matter of creating or modifying an "entity".
#[derive(Clone,Display, Serialize, Deserialize)]
#[display(fmt="HearthstoneGameAction[TM: {}\tType: {}\tBlock: {:?}\tEntityId: {}\tTags: {:#?}\tAttributes: {:#?}]", tm, action_type, current_block_id, entity_id, tags, attributes)]
pub struct HearthstoneGameAction {
    // Time at which this action was performed
    pub tm: DateTime<Utc>,
    #[serde(rename = "actionType")]
    pub action_type: ActionType,
    // Which entity is this action referring to. It's either the
    // GameEntity (modifying game state), a player, a new entity, or an existing entity.
    #[serde(skip)]
    pub entity_id: EntityId,
    #[serde(rename = "currentBlockId")]
    pub current_block_id: Option<Uuid>,
    // Only set once used to advance the game snapshot.
    #[serde(rename = "realEntityId")]
    pub real_entity_id: Option<i32>,
    // Tags to apply to this specific entity.
    pub tags: HashMap<String, String>,
    // Attributes to apply to this specific entry.
    // Generally attributes are found on the same line as the action though it's generally
    // up to the action to determine what's a tag and what's an attribute.
    pub attributes: HashMap<String, String>
}

impl std::default::Default for HearthstoneGameAction {
    fn default() -> Self {
        Self {
            tm: Utc::now(),
            action_type: ActionType::TagChange,
            entity_id: EntityId::None,
            current_block_id: None,
            real_entity_id: None,
            tags: HashMap::new(),
            attributes: HashMap::new()
        }
    }
}

#[derive(Clone,Display,Debug,Serialize)]
#[display(fmt="HearthstoneEntity[EntityId: {}\tTags: {:#?}\tAttributes: {:#?}]", entity_id, tags, attributes)]
pub struct HearthstoneEntity {
    #[serde(rename = "entityId")]
    pub entity_id: i32,
    pub tags: HashMap<String, String>,
    pub attributes: HashMap<String, String>
}

impl HearthstoneEntity {
    pub fn play_state(&self) -> play_state::PlayState {
        match self.tags.get("PLAYSTATE") {
            Some(playstate) => play_state::PlayState::from_str(playstate).unwrap_or(play_state::PlayState::Invalid),
            None => play_state::PlayState::Invalid
        }
    }
}

#[derive(Clone,Display,Debug,Serialize)]
#[display(fmt="HearthstoneGameSnapshotAuxData[]")]
pub struct HearthstoneGameSnapshotAuxData {
    #[serde(rename = "currentTurn")]
    pub current_turn: i32,
    pub step: game_step::GameStep,
    #[serde(rename = "currentPlayerId")]
    pub current_player_id: i32,
    #[serde(rename = "lastActionIndex")]
    pub last_action_index: usize
}

#[derive(Clone,Display,Serialize)]
#[display(fmt="HearthstoneGameSnapshot[Uuid: {}\tTime: {:?}\tGameEntityId: {}\tNameToPlayerId: {:#?}\tPlayerIdToEntityId: {:#?}\tEntities: {:#?}\tAux Data {:?}]", uuid, tm, game_entity_id, player_name_to_player_id, player_id_to_entity_id, entities, aux_data)]
pub struct HearthstoneGameSnapshot {
    pub uuid: Uuid,
    pub tm: Option<DateTime<Utc>>,
    // The ID of the entity to find when the entityId is "GameEntity"
    #[serde(rename = "gameEntityId")]
    pub game_entity_id: i32,
    // Map to go from player name/tag => Player ID => Entity ID.
    #[serde(rename = "playerNameToPlayerId")]
    pub player_name_to_player_id: HashMap<String, i32>,
    #[serde(rename = "playerIdToEntityId")]
    pub player_id_to_entity_id: HashMap<i32, i32>,
    // All entities indexed using their entity ID.
    pub entities: HashMap<i32, HearthstoneEntity>,
    #[serde(rename = "auxData")]
    pub aux_data: Option<HearthstoneGameSnapshotAuxData>
}

impl HearthstoneGameSnapshot {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            tm: None,
            game_entity_id: 0,
            player_name_to_player_id: HashMap::new(),
            player_id_to_entity_id: HashMap::new(),
            entities: HashMap::new(),
            aux_data: None,
        }
    }

    fn extract_aux_data(&mut self, last_action_index: usize) {
        let entity = game_entity::GameEntity::new(self.get_game_entity());
        let mut current_player_id = 0;

        // Needs the clone because rust doesn't like the reference borrow for player_id_to_entity_id (and thus self)
        // coupled with the mutable borrow of self in get_entity_from_id.
        for (pid, eid) in self.player_id_to_entity_id.clone() {
            let player = player_entity::PlayerEntity::new(self.get_entity_from_id(eid));
            if player.is_current_player() {
                current_player_id = pid;
                break;
            }
        }

        self.aux_data = Some(HearthstoneGameSnapshotAuxData{
            current_turn: entity.current_turn() ,
            step: entity.current_step(),
            current_player_id: current_player_id,
            last_action_index: last_action_index
        });
    }

    fn entity_from_entity_id(&mut self, entity_id: &EntityId) -> Option<&mut HearthstoneEntity> {
        match entity_id {
            EntityId::GameEntity => self.get_game_entity(),
            EntityId::NewGameEntity(entity_id) => self.create_game_entity(*entity_id),
            EntityId::Player(name) => self.get_player_entity(name.clone()),
            EntityId::NewPlayer{entity_id, player_id} => self.create_player_entity(*entity_id, *player_id),
            EntityId::New(id) => self.create_entity(*id),
            EntityId::Existing(id) => self.get_entity_from_generic_id(id),
            _ => None
        }
    }

    fn advance(&mut self, action: &mut HearthstoneGameAction) {
        self.tm = Some(action.tm.clone());
        let entity = self.entity_from_entity_id(&action.entity_id);

        if entity.is_none() {
            log::warn!("Unknown Entity: {}", &action.entity_id);
            return;
        }

        let entity = entity.unwrap();
        action.real_entity_id = Some(entity.entity_id);
        // Merge tags and attributes into the entity.
        for (key, value) in &action.tags {
            entity.tags.insert(key.clone(), value.clone());
        }

        for (key, value) in &action.attributes {
            entity.attributes.insert(key.clone(), value.clone());
        }
    }

    fn get_entity_from_id(&mut self, id: i32) -> Option<&mut HearthstoneEntity> {
        self.entities.get_mut(&id)
    }

    fn get_entity_from_generic_id(&mut self, id: &str) -> Option<&mut HearthstoneEntity> {
        // The EntityID can come in a number of forms:
        // - "GameEntity" -> just the plain old game entity.
        // - "UNKNOWN HUMAN PLAYER" -> this is a player that we don't know the name of yet!
        // - "NAME#TAG" -> this is a Battle.net tag that indicates a *player*
        // - "NUMBER" -> A raw entity ID
        // - "[... id=ID]" -> An entity object.
        if id == "GameEntity" {
            self.get_game_entity()
        } else if id.chars().all(char::is_numeric) {
            let id: i32 = id.parse().unwrap_or(-1);
            self.get_entity_from_id(id)
        } else if id.find('[').is_some() && id.find(']').is_some() {
            lazy_static! {
                static ref RE: Regex = Regex::new("\\[.*id=(.*?)\\s.*\\]").unwrap();
            }

            let captures = match RE.captures(id) {
                Some(x) => x,
                None => return None
            };

            let entity_id : i32 = captures.get(1).map_or("-1", |m| m.as_str()).parse().unwrap_or(-1);
            self.get_entity_from_id(entity_id)
        } else if id == "UNKNOWN HUMAN PLAYER" || id.find('#').is_some() || id == "Bob's Tavern" {
            self.get_player_entity(String::from(id))
        } else {
            // In Battlegrounds, we'll encounter names that we haven't seen before when we start vsing other
            // players...In some cases, it'll set CURRENT_PLAYER on Bob's Tavern and on other times it'll set
            // CURRENT_PLAYER on this other player. Honestly no idea why so we're going to pretend in those cases
            // that the game wrote Bob's Tavern instead.
            self.get_player_entity(String::from("Bob's Tavern"))
        }
    }

    fn create_entity(&mut self, id: i32) -> Option<&mut HearthstoneEntity> {
        let new_entity = HearthstoneEntity{
            entity_id: id,
            tags: HashMap::new(),
            attributes: HashMap::new(),
        };
        self.entities.insert(id, new_entity);
        self.get_entity_from_id(id)
    }

    fn get_game_entity(&mut self) -> Option<&mut HearthstoneEntity> {
        self.get_entity_from_id(self.game_entity_id)
    }

    fn create_game_entity(&mut self, id: i32) -> Option<&mut HearthstoneEntity> {
        self.game_entity_id = id;
        self.create_entity(id)
    }

    pub fn get_match_winner_player_id(&self) -> Option<i32> {
        for (pid, eid) in &self.player_id_to_entity_id {
            let entity = self.entities.get(eid);
            if entity.is_none() {
                continue
            }
            let entity = entity.unwrap();
            let playstate = entity.play_state();
            if playstate == play_state::PlayState::Won {
                return Some(*pid)
            }
        }
        None
    }

    fn get_player_entity(&mut self, player_name: String) -> Option<&mut HearthstoneEntity> {
        // There's a couple of possibilities here:
        // 1) The player name exists in our player_name_to_player_id map, proceeding is straightforward.
        // 2) The player's name does not exist! In which case
        //  a) UNKNOWN HUMAN PLAYER should exist in the map instead. We can replace UNKNOWN HUMAN PLAYER with the new name we found.
        //  b) UNKNOWN HUMAN PLAYER doesn't exist in which case wtf.
        let mut need_replacement: Option<String> = None;
        let player_id = match self.player_name_to_player_id.get(&player_name) {
            Some(x) => Some(x),
            None => {
                if player_name == "Bob's Tavern" {
                    self.player_name_to_player_id.get("The Innkeeper")
                } else {
                    need_replacement = Some(String::from("UNKNOWN HUMAN PLAYER"));
                    self.player_name_to_player_id.get("UNKNOWN HUMAN PLAYER")
                }
            }
        };

        if player_id.is_none() {
            return None;
        }

        let player_id = *player_id.unwrap();
        if need_replacement.is_some() {
            let need_replacement = need_replacement.unwrap();
            self.player_name_to_player_id.insert(player_name.clone(), player_id);
            self.player_name_to_player_id.remove(&need_replacement);
        }

        let entity_id = match self.player_id_to_entity_id.get(&player_id) {
            Some(x) => x,
            None => return None
        }.clone();

        self.get_entity_from_id(entity_id)
    }

    fn create_player_entity(&mut self, id: i32, player_id: i32) -> Option<&mut HearthstoneEntity> {
        self.player_id_to_entity_id.insert(player_id, id);
        self.create_entity(id)
    }

    fn set_player_map(&mut self, m: &HashMap<i32, String>) {
        for (id, name) in m {
            self.player_name_to_player_id.insert(name.clone(), *id);
        }
    }
}

pub struct HearthstoneGameLog {
    pub current_state: HearthstoneGameSnapshot,
    pub snapshots: Vec<HearthstoneGameSnapshot>,
    pub actions: Vec<HearthstoneGameAction>,
    pub blocks: HashMap<Uuid, HearthstoneGameBlock>,
    pub current_blocks: Vec<Uuid>
}

impl HearthstoneGameLog {
    pub fn new() -> Self {
        Self {
            current_state: HearthstoneGameSnapshot::new(),
            snapshots: Vec::new(),
            actions: Vec::new(),
            blocks: HashMap::new(),
            current_blocks: Vec::new(),
        }
    }

    pub fn push_block(&mut self, block_type: BlockType, entity_id: &EntityId) {
        let entity = self.current_state.entity_from_entity_id(entity_id);

        let block = HearthstoneGameBlock{
            block_id: Uuid::new_v4(),
            // We'll use end_action_index < start_action_index as indicating an empty block.
            start_action_index: self.actions.len() as i32,
            end_action_index: 0,
            block_type: block_type,
            parent_block: self.current_blocks.last().copied(),
            entity_id: match entity {
                Some(x) => x.entity_id,
                None => 0
            }
        };

        self.current_blocks.push(block.block_id.clone());
        self.blocks.insert(block.block_id, block);
    }

    pub fn pop_block(&mut self) {
        match self.current_blocks.pop() {
            Some(x) => {
                let block = self.blocks.get_mut(&x).unwrap();
                block.end_action_index = (self.actions.len() - 1) as i32;
                ()
            },
            None => ()
        };
    }

    pub fn advance(&mut self, actions: Vec<HearthstoneGameAction>) {
        // Compare the old state vs the new state to see if we should take a snapshot.
        // Namely, we want to keep a snapshot every time the turn updates.
        let old_game_entity = game_entity::GameEntity::new(self.current_state.get_game_entity());

        for mut a in actions {
            a.current_block_id = self.current_blocks.last().copied();
            self.current_state.advance(&mut a);
            self.actions.push(a);
        }

        let new_game_entity = game_entity::GameEntity::new(self.current_state.get_game_entity());
        // Check if we're about to advance to the next turn
        if (new_game_entity.current_step() == game_step::GameStep::MainNext && new_game_entity.current_step() != old_game_entity.current_step()) ||
        // Check if we advanced from mulligan to the first turn
            old_game_entity.simple_step() != new_game_entity.simple_step() {
            self.create_new_snapshot();
        }
    }

    pub fn create_new_snapshot(&mut self) {
        // Create a copy of the current state of the game and push it onto the snapshot list.
        // Furthermore, at this point we want to extract certain information out of the snapshot
        // that'll be useful for us in presenting information to the user.
        let mut new_snapshot = self.current_state.clone();
        new_snapshot.uuid = Uuid::new_v4();
        new_snapshot.extract_aux_data(self.actions.len() - 1);
        self.snapshots.push(new_snapshot);
    }

    pub fn finish(&mut self) {
        self.create_new_snapshot();
    }

    pub fn set_player_map(&mut self, m: &HashMap<i32, String>) {
        self.current_state.set_player_map(m);
    }
}