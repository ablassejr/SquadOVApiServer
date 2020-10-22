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