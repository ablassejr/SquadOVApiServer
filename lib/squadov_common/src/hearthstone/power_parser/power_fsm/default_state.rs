use std::rc::Rc;
use std::cell::RefCell;
use crate::hearthstone::game_state::HearthstoneGameLog;
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmAction};
use uuid::Uuid;

// We use the DefaultPowerState to keep track of game snapshots and game actions.
pub struct DefaultPowerState {
    game: Rc<RefCell<HearthstoneGameLog>>,
    uuid: Uuid
}

impl PowerFsmState for DefaultPowerState {
    fn on_enter_state_from_child(&mut self, previous: &mut Box<dyn PowerFsmState>) {
        let action = previous.generate_hearthstone_game_actions();
        if action.is_some() {
            self.game.borrow_mut().advance(action.unwrap());
        }
    }

    fn get_state_uuid(&self) -> Uuid {
        self.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::Unknown
    }
}
impl DefaultPowerState {
    pub fn new(st: Rc<RefCell<HearthstoneGameLog>>) -> Self {
        Self {
            game: st.clone(),
            uuid: Uuid::new_v4(),
        }
    }
}