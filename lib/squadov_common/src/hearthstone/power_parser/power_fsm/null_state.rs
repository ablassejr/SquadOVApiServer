use uuid::Uuid;
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmAction};

// Null State for when this is technically an action that exists but we don't
// particularly care about it.
pub struct NullState {
    uuid: Uuid
}

impl PowerFsmState for NullState {
    fn get_state_uuid(&self) -> Uuid {
        self.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::Unknown
    }
}

impl NullState {
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }
}