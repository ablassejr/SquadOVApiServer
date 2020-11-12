use derive_more::{Display};
use serde_repr::Serialize_repr;
use num_enum::TryFromPrimitive;

#[derive(sqlx::Type, Display, Clone, Copy, Serialize_repr, TryFromPrimitive)]
#[repr(i32)]
pub enum FormatType {
    Unknown,
    Wild,
    Standard
}

impl std::str::FromStr for FormatType {
    type Err = crate::SquadOvError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "FT_UNKNOWN" => FormatType::Unknown,
            "FT_WILD" => FormatType::Wild,
            "FT_STANDARD" => FormatType::Standard,
            _ => FormatType::Unknown
        })
    }
}