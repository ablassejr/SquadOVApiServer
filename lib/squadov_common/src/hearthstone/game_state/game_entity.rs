use crate::hearthstone::game_state::HearthstoneEntity;
use crate::hearthstone::game_state::game_step::{GameStep, SimpleGameStep};

pub struct GameEntity {
    turn: i32,
    step: GameStep
}

impl GameEntity {
    pub fn new(entity: Option<&mut HearthstoneEntity>) -> Self {
        match entity {
            Some(e) => Self {
                turn: match e.tags.get("TURN") {
                    Some(x) => x.parse().unwrap_or(-1),
                    None => -1
                },
                step: match e.tags.get("STEP") {
                    Some(x) => x.parse().unwrap_or(GameStep::Invalid),
                    None => GameStep::Invalid
                }
            },
            None => Self {
                turn: -1,
                step: GameStep::Invalid
            }
        }
    }

    pub fn current_turn(&self) -> i32 {
        self.turn
    }

    pub fn current_step(&self) -> GameStep {
        self.step
    }

    pub fn simple_step(&self) -> SimpleGameStep {
        self.current_step().into()
    }
}