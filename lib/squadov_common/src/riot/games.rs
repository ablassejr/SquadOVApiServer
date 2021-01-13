pub mod valorant;
pub mod lol;
pub mod tft;

pub use valorant::*;
pub use lol::*;
pub use tft::*;

pub const VALORANT_SHORTHAND: &'static str = "val";
pub const LOL_SHORTHAND: &'static str = "lol";
pub const TFT_SHORTHAND: &'static str = "tft";