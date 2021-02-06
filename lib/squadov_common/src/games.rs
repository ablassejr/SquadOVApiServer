use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Copy, Clone, Serialize_repr, Deserialize_repr, Debug)]
#[repr(i32)]
pub enum SquadOvGames {
    AimLab,
    Hearthstone,
    LeagueOfLegends,
    TeamfightTactics,
    Valorant,
    WorldOfWarcraft,
}