use crate::hearthstone::game_state::{HearthstoneGameAction, EntityId, ActionType};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmStateInfo, PowerFsmAction};
use uuid::Uuid;

pub struct TagChangeState {
    info: PowerFsmStateInfo,
}

impl PowerFsmState for TagChangeState {   
    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::TagChange
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        Some(vec![
            HearthstoneGameAction {
                tm: self.info.tm.clone(),
                action_type: ActionType::TagChange,
                entity_id: EntityId::Existing(self.info.attrs.get("Entity").unwrap().to_string()),
                real_entity_id: None,
                current_block_id: None,
                tags: self.info.tags.clone(),
                attributes: self.info.attrs.clone()
            }
        ])
    }
}

impl TagChangeState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        let mut ret = Self {
            info,
        };

        // Tag changes are a bit weird in that the tags will be confused as attributes  by default
        // since the logs are on a single line.
        let tag = ret.info.attrs.get("tag");
        let value = ret.info.attrs.get("value");

        if tag.is_some() && value.is_some() {
            ret.info.tags.insert(tag.unwrap().clone(), value.unwrap().clone());
        }

        ret.info.attrs.remove("tag");
        ret.info.attrs.remove("value");

        ret
    }
}