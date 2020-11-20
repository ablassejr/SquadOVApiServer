use crate::SquadOvError;
use derive_more::{Display};
use num_enum::TryFromPrimitive;
use serde_repr::Serialize_repr;

#[derive(Display, Debug, Clone, Copy, PartialEq, Serialize_repr, TryFromPrimitive)]
#[repr(i32)]
pub enum PlayState {
    #[display(fmt = "INVALID")]
    Invalid,
    #[display(fmt = "PLAYING")]
    Playing,
    #[display(fmt = "WINNING")]
    Winning,
    #[display(fmt = "LOSING")]
    Losing,
    #[display(fmt = "WON")]
    Won,
    #[display(fmt = "LOST")]
    Lost,
    #[display(fmt = "TIED")]
    Tied,
    #[display(fmt = "DISCONNECTED")]
    Disconnected,
    #[display(fmt = "CONCEDED")]
	Conceded
}

impl std::str::FromStr for PlayState {
    type Err = SquadOvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "PLAYING" => PlayState::Playing,
            "WINNING" => PlayState::Winning,
            "LOSING" => PlayState::Losing,
            "WON" => PlayState::Won,
            "LOST" => PlayState::Lost,
            "TIED" => PlayState::Tied,
            "DISCONNECTED" => PlayState::Disconnected,
            "CONCEDED" => PlayState::Conceded,
            _ => PlayState::Invalid,
        })
    }
}