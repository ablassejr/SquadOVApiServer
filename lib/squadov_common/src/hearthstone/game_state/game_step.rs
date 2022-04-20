use crate::SquadOvError;
use derive_more::{Display};
use num_enum::TryFromPrimitive;
use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Display, Debug, Clone, Copy, PartialEq, Serialize_repr, Deserialize_repr, TryFromPrimitive)]
#[repr(i32)]
pub enum GameStep {
    #[display(fmt = "INVALID")]
    Invalid,
    #[display(fmt = "BEGIN_FIRST")]
    BeginFirst,
    #[display(fmt = "BEGIN_SHUFFLE")]
    BeginShuffle,
    #[display(fmt = "BEGIN_DRAW")]
    BeginDraw,
    #[display(fmt = "BEGIN_MULLIGAN")]
    BeginMulligan,
    #[display(fmt = "MAIN_BEGIN")]
    MainBegin,
    #[display(fmt = "MAIN_READY")]
    MainReady,
    #[display(fmt = "MAIN_RESOURCE")]
    MainResource,
    #[display(fmt = "MAIN_DRAW")]
    MainDraw,
    #[display(fmt = "MAIN_START")]
    MainStart,
    #[display(fmt = "MAIN_ACTION")]
    MainAction,
    #[display(fmt = "MAIN_COMBAT")]
    MainCombat,
    #[display(fmt = "MAIN_END")]
    MainEnd,
    #[display(fmt = "MAIN_NEXT")]
    MainNext,
    #[display(fmt = "FINAL_WRAPUP")]
    FinalWrapup,
    #[display(fmt = "FINAL_GAMEOVER")]
    FinalGameover,
    #[display(fmt = "MAIN_CLEANUP")]
    MainCleanup,
    #[display(fmt = "MAIN_START_TRIGGER")]
	MainStartTriggers
}

// We use the simple game step to boil down these steps into more general phases
// for more simplified processing.
#[derive(PartialEq)]
pub enum SimpleGameStep {
    Invalid,
    Mulligan,
    Play,
    Finish
}

impl From<GameStep> for SimpleGameStep {
    fn from(s : GameStep) -> Self {
        match s {
            GameStep::Invalid => SimpleGameStep::Invalid,
            GameStep::BeginMulligan => SimpleGameStep::Mulligan,
            GameStep::FinalWrapup | GameStep::FinalGameover => SimpleGameStep::Finish,
            _ => SimpleGameStep::Play
        }
    }
}

impl std::str::FromStr for GameStep {
    type Err = SquadOvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BEGIN_FIRST" => GameStep::BeginFirst,
            "BEGIN_SHUFFLE" => GameStep::BeginShuffle,
            "BEGIN_DRAW" => GameStep::BeginDraw,
            "BEGIN_MULLIGAN" => GameStep::BeginMulligan,
            "MAIN_BEGIN" => GameStep::MainBegin,
            "MAIN_READY" => GameStep::MainReady,
            "MAIN_RESOURCE" => GameStep::MainResource,
            "MAIN_DRAW" => GameStep::MainDraw,
            "MAIN_START" => GameStep::MainStart,
            "MAIN_ACTION" => GameStep::MainAction,
            "MAIN_COMBAT" => GameStep::MainCombat,
            "MAIN_END" => GameStep::MainEnd,
            "MAIN_NEXT" => GameStep::MainNext,
            "FINAL_WRAPUP" => GameStep::FinalWrapup,
            "FINAL_GAMEOVER" => GameStep::FinalGameover,
            "MAIN_CLEANUP" => GameStep::MainCleanup,
            "MAIN_START_TRIGGERS" => GameStep::MainStartTriggers,
            _ => GameStep::Invalid,
        })
    }
}