use derive_more::{Display};

#[derive(Display)]
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