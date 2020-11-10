mod game_entity;

use derive_more::{Display};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use regex::Regex;

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
    Existing(String)
}

// Generally actions are just a matter of creating or modifying an "entity".
#[derive(Clone,Display)]
#[display(fmt="HearthstoneGameAction[TM: {}\tEntityId: {}\tTags: {:#?}\tAttributes: {:#?}]", tm, entity_id, tags, attributes)]
pub struct HearthstoneGameAction {
    // Time at which this action was performed
    pub tm: DateTime<Utc>,
    // Which entity is this action referring to. It's either the
    // GameEntity (modifying game state), a player, a new entity, or an existing entity.
    pub entity_id: EntityId,
    // Tags to apply to this specific entity.
    pub tags: HashMap<String, String>,
    // Attributes to apply to this specific entry.
    // Generally attributes are found on the same line as the action though it's generally
    // up to the action to determine what's a tag and what's an attribute.
    pub attributes: HashMap<String, String>
}

#[derive(Clone,Display,Debug)]
#[display(fmt="HearthstoneEntity[EntityId: {}\tTags: {:#?}\tAttributes: {:#?}]", entity_id, tags, attributes)]
pub struct HearthstoneEntity {
    entity_id: i32,
    pub tags: HashMap<String, String>,
    pub attributes: HashMap<String, String>
}

#[derive(Clone,Display)]
#[display(fmt="HearthstoneGameSnapshot[GameEntityId: {}\tNameToPlayerId: {:#?}\tPlayerIdToEntityId: {:#?}\tEntities: {:#?}]", game_entity_id, player_name_to_player_id, player_id_to_entity_id, entities)]
pub struct HearthstoneGameSnapshot {
    // The ID of the entity to find when the entityId is "GameEntity"
    game_entity_id: i32,
    // Map to go from player name/tag => Player ID => Entity ID.
    player_name_to_player_id: HashMap<String, i32>,
    player_id_to_entity_id: HashMap<i32, i32>,
    // All entities indexed using their entity ID.
    entities: HashMap<i32, HearthstoneEntity>
}

impl HearthstoneGameSnapshot {
    pub fn new() -> Self {
        Self {
            game_entity_id: 0,
            player_name_to_player_id: HashMap::new(),
            player_id_to_entity_id: HashMap::new(),
            entities: HashMap::new()
        }
    }

    fn advance(&mut self, action: &HearthstoneGameAction) {
        let entity = match &action.entity_id {
            EntityId::GameEntity => self.get_game_entity(),
            EntityId::NewGameEntity(entity_id) => self.create_game_entity(*entity_id),
            EntityId::Player(name) => self.get_player_entity(name.clone()),
            EntityId::NewPlayer{entity_id, player_id} => self.create_player_entity(*entity_id, *player_id),
            EntityId::New(id) => self.create_entity(*id),
            EntityId::Existing(id) => self.get_entity_from_generic_id(id)
        };

        if entity.is_none() {
            log::warn!("Unknown Entity: {}", &action.entity_id);
            return;
        }

        let entity = entity.unwrap();
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
        } else if id == "UNKNOWN HUMAN PLAYER" || id.find('#').is_some() {
            self.get_player_entity(String::from(id))
        } else {
            None
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

    fn get_player_entity(&mut self, player_name: String) -> Option<&mut HearthstoneEntity> {
        // There's a couple of possibilities here:
        // 1) The player name exists in our player_name_to_player_id map, proceeding is straightforward.
        // 2) The player's name does not exist! In which case
        //  a) UNKNOWN HUMAN PLAYER should exist in the map instead. We can replace UNKNOWN HUMAN PLAYER with the new name we found.
        //  b) UNKNOWN HUMAN PLAYER doesn't exist in which case wtf.
        let player_id = match self.player_name_to_player_id.get(&player_name) {
            Some(x) => Some(x),
            None => self.player_name_to_player_id.get("UNKNOWN HUMAN PLAYER")
        };

        if player_id.is_none() {
            return None;
        }

        let player_id = player_id.unwrap();
        let entity_id = match self.player_id_to_entity_id.get(player_id) {
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
    pub actions: Vec<HearthstoneGameAction>
}

impl HearthstoneGameLog {
    pub fn new() -> Self {
        Self {
            current_state: HearthstoneGameSnapshot::new(),
            snapshots: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn advance(&mut self, actions: Vec<HearthstoneGameAction>) {
        // Compare the old state vs the new state to see if we should take a snapshot.
        // Namely, we want to keep a snapshot every time the turn updates.
        let old_game_entity = game_entity::GameEntity::new(self.current_state.get_game_entity());

        for a in actions {
            self.current_state.advance(&a);
            self.actions.push(a);
        }

        let new_game_entity = game_entity::GameEntity::new(self.current_state.get_game_entity());
        if old_game_entity.current_turn() != new_game_entity.current_turn() {
            self.snapshots.push(self.current_state.clone());
        }
    }

    pub fn set_player_map(&mut self, m: &HashMap<i32, String>) {
        self.current_state.set_player_map(m);
    }
}