use chrono::{DateTime, SecondsFormat};

pub fn sql_format_bool(v: bool) -> &'static str {
    if v {
        "TRUE"
    } else {
        "FALSE"
    }
}

pub fn sql_format_option_string<T>(v: &Option<T>) -> String
where T: std::fmt::Display
{
    match v {
        Some(x) => format!("'{}'", x),
        None => String::from("NULL")
    }
}

pub fn sql_format_option_value<T>(v: &Option<T>) -> String
where T: std::fmt::Display
{
    match v {
        Some(x) => format!("{}", x),
        None => String::from("NULL")
    }
}

pub fn sql_format_option_some_time<T>(v: Option<&DateTime<T>>) -> String
where T: chrono::TimeZone,
      <T as chrono::TimeZone>::Offset: std::fmt::Display
{
    match v {
        Some(x) => format!("'{}'", x.to_rfc3339_opts(SecondsFormat::Micros, true)),
        None => String::from("NULL")
    }
}