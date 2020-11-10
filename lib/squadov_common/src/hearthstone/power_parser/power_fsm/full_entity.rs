use crate::hearthstone::game_state::{HearthstoneGameAction, EntityId};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmStateInfo, PowerFsmAction};
use uuid::Uuid;

pub struct FullEntityState {
    info: PowerFsmStateInfo,
}

impl PowerFsmState for FullEntityState {
    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::FullEntity
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        Some(vec![
            HearthstoneGameAction {
                tm: self.info.tm.clone(),
                entity_id: EntityId::New(self.info.attrs.get("ID").unwrap().parse().unwrap()),
                tags: self.info.tags.clone(),
                attributes: self.info.attrs.clone()
            }
        ])
    }
}

impl FullEntityState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        Self {
            info,
        }
    }
}