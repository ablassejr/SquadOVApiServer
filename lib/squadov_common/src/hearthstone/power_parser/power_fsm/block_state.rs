use crate::hearthstone::game_state::{HearthstoneGameAction, HearthstoneGameLog, BlockType};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmStateInfo, PowerFsmAction, is_fsm_action_block_end};
use uuid::Uuid;
use std::rc::Rc;
use std::cell::RefCell;

pub struct BlockState {
    info: PowerFsmStateInfo,
    game: Rc<RefCell<HearthstoneGameLog>>,
    block_type: BlockType,
    has_actions: bool
}

impl PowerFsmState for BlockState {
    fn on_enter_state_from_parent(&mut self, _previous: &mut Box<dyn PowerFsmState>) {
        // Start Block
        self.game.borrow_mut().push_block(self.block_type);
    }

    fn on_enter_state_from_child(&mut self, previous: &mut Box<dyn PowerFsmState>) {
        // Need to immediately pass actions from the child to the game log.
        // Note that this is similar to the default state, in fact the default state can
        // be thought of as just a stripped block state.
        let action = previous.generate_hearthstone_game_actions();
        if action.is_some() {
            self.game.borrow_mut().advance(action.unwrap());
        }
        self.has_actions = true;
    }

    fn on_leave_state_to_parent(&mut self) {
        // End Block
        self.game.borrow_mut().pop_block();
    }

    fn generate_hearthstone_game_actions(&self) -> Option<Vec<HearthstoneGameAction>> {
        // Needs to be none here because we handle adding actions to the game log ourself
        // instead of letting the default state handle it.
        None
    }

    fn get_state_uuid(&self) -> Uuid {
        self.info.uuid
    }

    fn get_state_action(&self) -> PowerFsmAction {
        PowerFsmAction::BlockStart
    }

    // If can_receive_action is false, then we'll do a pop, if it's true then
    // we won't do a pop. We need some special logic here because is we receive an
    // is_fsm_action_block_end action we want to pop the state either once or twice. We want
    // to do it once if this block has received no actions and twice if this block has
    // received actions. There's a guaranteed pop by the FSM so this is the 2nd pop
    // when we receive a block end.
    fn can_receive_action(&self, action: &PowerFsmAction) -> bool {
        !is_fsm_action_block_end(action) && !self.has_actions
     }
}
impl BlockState {
    pub fn new(info: PowerFsmStateInfo, st: Rc<RefCell<HearthstoneGameLog>>) -> Self {
        let block_type = match info.attrs.get("BlockType") {
            Some(x) => x.parse().unwrap_or(BlockType::Invalid),
            None => BlockType::Invalid,
        };
        Self {
            info,
            game: st.clone(),
            block_type: block_type,
            has_actions: false
        }
    }
}