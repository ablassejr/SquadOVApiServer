use serde_repr::{Serialize_repr, Deserialize_repr};
use num_enum::TryFromPrimitive;

#[derive(Copy, Clone, Serialize_repr, Deserialize_repr, Debug, TryFromPrimitive, PartialEq)]
#[repr(i32)]
pub enum SquadOvGames {
    AimLab,
    Hearthstone,
    LeagueOfLegends,
    TeamfightTactics,
    Valorant,
    WorldOfWarcraft,
    Csgo,
    Unknown,
}