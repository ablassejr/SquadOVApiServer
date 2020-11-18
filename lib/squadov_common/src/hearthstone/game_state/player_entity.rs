use crate::hearthstone::game_state::HearthstoneEntity;

pub struct PlayerEntity {
    current_player: bool
}

impl PlayerEntity {
    pub fn new(entity: Option<&mut HearthstoneEntity>) -> Self {
        match entity {
            Some(e) => Self{
                current_player: match e.tags.get("CURRENT_PLAYER") {
                    Some(x) => x.parse::<i32>().unwrap_or(0) == 1,
                    None => false
                }
            },
            None => Self{
                current_player: false
            }
        }
    }

    pub fn is_current_player(&self) -> bool {
        self.current_player
    }
}