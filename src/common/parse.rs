use chrono::{DateTime, Utc, TimeZone};
use serde::{Deserializer};
use serde::de::{self, Visitor, Unexpected};
use std::fmt;
use std::convert::TryFrom;

pub fn parse_utc_time_from_milliseconds<'a, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'a>,
{
    struct UnixTimeVisitor;

    impl<'a> Visitor<'a> for UnixTimeVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "an integer representing the number of milliseconds since Unix epoch")
        }

        fn visit_u64<E>(self, v : u64) -> Result<Self::Value, E>
        where 
            E: de::Error
        {
            let i = match i64::try_from(v) {
                Ok(i) => i,
                Err(_) => return Err(de::Error::invalid_value(Unexpected::Unsigned(v), &self)),
            };

            Ok(Some(Utc.timestamp_millis(i)))
        }
    }

    deserializer.deserialize_u64(UnixTimeVisitor{})
}