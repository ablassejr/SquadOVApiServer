use serde::{
    Serialize,
    Serializer,
    Deserialize,
    Deserializer,
    de::Error,
};
use derive_more::{Display};

#[derive(Display, Clone, PartialEq, Eq, Hash)]
pub enum StripeCurrency {
    #[display(fmt="usd")]
    Usd,
    #[display(fmt="eur")]
    Euro,
}

impl Default for StripeCurrency {
    fn default() -> Self {
        StripeCurrency::Usd
    }
}

impl Serialize for StripeCurrency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StripeCurrency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "usd" => StripeCurrency::Usd,
            "eur" => StripeCurrency::Euro,
            _ => return Err(D::Error::custom("Invalid")),
        })
    }
}