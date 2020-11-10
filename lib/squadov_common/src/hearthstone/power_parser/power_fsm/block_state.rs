use crate::hearthstone::game_state::{ HearthstoneGameAction};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmStateInfo, PowerFsmAction};
use uuid::Uuid;

pub struct BlockState {
    info: PowerFsmStateInfo,
    actions: Vec<HearthstoneGameAction>,
}

impl PowerFsmState for BlockState {
    fn on_enter_state_from_child(&mut self, previous: &mut Box<dyn PowerFsmState>) {
        let action = previous.generate_hearthstone_game_actions();
        if action.is_some() {
            self.actions.extend(action.unwrap());
        }
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        Some(self.actions.clone())
    }

    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::BlockStart
    }
}
impl BlockState {
    pub fn new(info: PowerFsmStateInfo) -> Self {
        Self {
            info,
            actions: Vec::new(),
        }
    }
}