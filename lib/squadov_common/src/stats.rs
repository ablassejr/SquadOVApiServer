mod aimlab;

pub use aimlab::*;

use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Copy, Clone, Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum StatPermission {
    AimlabGridshot,
    AimlabSpidershot,
    AimlabMicroshot,
    AimlabSixshot,
    AimlabMicroflex,
    AimlabMotionshot,
    AimlabMultishot,
    AimlabDetection,
    AimlabDecisionshot,
    AimlabStrafetrack,
    AimlabCircletrack,
    AimlabStrafeshot,
    AimlabCircleshot,
    AimlabLinetrace,
    AimlabMultilinetrace,
    AimlabPentakill,
}