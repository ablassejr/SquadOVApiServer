use crate::hearthstone::game_state::{HearthstoneGameAction, EntityId, HearthstoneGameLog, BlockType};
use crate::hearthstone::power_parser::power_fsm::{PowerFsmState, PowerFsmStateInfo, PowerFsmAction, is_fsm_action_block_end};
use uuid::Uuid;
use std::sync::{Arc, RwLock};

pub struct BlockState {
    info: PowerFsmStateInfo,
    game: Arc<RwLock<HearthstoneGameLog>>,
    block_type: BlockType,
    entity_id: EntityId,
    has_actions: bool,
    indent_level: i32
}

impl PowerFsmState for BlockState {
    fn on_enter_state_from_parent(&mut self, _previous: &mut Box<dyn PowerFsmState + Send + Sync>) -> Result<(), crate::SquadOvError> {
        // Start Block
        self.game.write()?.push_block(self.block_type, &self.entity_id);
        Ok(())
    }

    fn on_enter_state_from_child(&mut self, previous: &mut Box<dyn PowerFsmState + Send + Sync>) -> Result<(), crate::SquadOvError> {
        // Need to immediately pass actions from the child to the game log.
        // Note that this is similar to the default state, in fact the default state can
        // be thought of as just a stripped block state.
        let action = previous.generate_hearthstone_game_actions();
        if action.is_some() {
            self.game.write()?.advance(action.unwrap());
        }

        // Blocks clean themselves up so if we come from a block we don't have an
        // active action going on.
        if previous.get_state_action() != PowerFsmAction::BlockStart {
            self.has_actions = true;
        } else {
            self.has_actions = false;
        }
        Ok(())
    }

    fn on_leave_state_to_parent(&mut self) -> Result<(), crate::SquadOvError> {
        // End Block
        self.game.write()?.pop_block();
        Ok(())
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
    fn can_receive_action(&self, action: &PowerFsmAction, indent_level: i32) -> bool {
        // So the logic here is we want to POP when the it's a block end action AND we have subactions.
        // In that scenario we want to return false so the logical inverse there is that we want to return
        // true if we're either not a block end or we have no actions.
        !(is_fsm_action_block_end(action) || self.is_implicit_block_end(action, indent_level)) || !self.has_actions
    }

    // Sometimes Hearthstone skips BLOCK_END so in that case we need to return true here when we detect that case.
    fn is_implicit_block_end(&self, action: &PowerFsmAction, indent_level: i32) -> bool {
        indent_level == self.indent_level && !is_fsm_action_block_end(action)
    }
}
impl BlockState {
    pub fn new(info: PowerFsmStateInfo, st: Arc<RwLock<HearthstoneGameLog>>, indent_level: i32) -> Self {
        let block_type = match &info.attrs.get("BlockType") {
            Some(x) => x.parse().unwrap_or(BlockType::Invalid),
            None => BlockType::Invalid,
        };

        let entity = EntityId::Existing(info.attrs.get("Entity").unwrap().to_string());
        Self {
            info,
            game: st.clone(),
            block_type: block_type,
            entity_id: entity,
            has_actions: false,
            indent_level
        }
    }
}