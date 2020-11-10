use crate::hearthstone::game_state::HearthstoneEntity;

pub struct GameEntity {
    // Turn number
    turn: i32
}

impl GameEntity {
    pub fn new(entity: Option<&mut HearthstoneEntity>) -> Self {
        match entity {
            Some(e) => Self {
                turn: match e.tags.get("TURN") {
                    Some(x) => x.parse().unwrap_or(-1),
                    None => -1
                }
            },
            None => Self {
                turn: -1
            }
        }
    }

    pub fn current_turn(&self) -> i32 {
        self.turn
    }
}