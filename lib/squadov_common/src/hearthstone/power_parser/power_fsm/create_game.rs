use crate::hearthstone::game_state::{HearthstoneGameAction, EntityId};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmAction, PowerFsmStateInfo};
use uuid::Uuid;

pub struct CreateGameState {
    info: PowerFsmStateInfo
}

impl PowerFsmState for CreateGameState {
    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::CreateGame
    }
}

impl CreateGameState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        Self {
            info,
        }
    }
}

pub struct CreateGameEntityState {
    info: PowerFsmStateInfo
}
impl PowerFsmState for CreateGameEntityState {
    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::CreateGameEntity
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        Some(vec![
            HearthstoneGameAction {
                tm: self.info.tm.clone(),
                entity_id: EntityId::NewGameEntity(
                    self.info.attrs.get("EntityID").unwrap().parse().unwrap()
                ),
                tags: self.info.tags.clone(),
                attributes: self.info.attrs.clone()
            }
        ])
    }
}

impl CreateGameEntityState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        Self {
            info,
        }
    }
}

pub struct CreateGamePlayerState {
    info: PowerFsmStateInfo
}
impl PowerFsmState for CreateGamePlayerState {
    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::CreatePlayerEntity
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        Some(vec![
            HearthstoneGameAction {
                tm: self.info.tm.clone(),
                entity_id: EntityId::NewPlayer{
                    entity_id: self.info.attrs.get("EntityID").unwrap().parse().unwrap(),
                    player_id: self.info.attrs.get("PlayerID").unwrap().parse().unwrap(),
                },
                tags: self.info.tags.clone(),
                attributes: self.info.attrs.clone()
            }
        ])
    }
}
impl CreateGamePlayerState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        Self {
            info,
        }
    }
}