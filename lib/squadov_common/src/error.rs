use actix_web::{error, HttpResponse, http::StatusCode, dev::HttpResponseBuilder};
use derive_more::{Display};
use sqlx;
use url;
use std::str;
use std::io;
use jsonwebtoken;
use reqwest;
use openssl;

#[derive(Debug, Display)]
pub enum SquadOvError {
    #[display(fmt = "[SquadovError] Invalid credentials.")]
    Credentials,
    #[display(fmt = "[SquadovError] Unauthorized Access")]
    Unauthorized,
    #[display(fmt = "[SquadovError] Invalid Request")]
    BadRequest,
    #[display(fmt = "[SquadovError] Not found")]
    NotFound,
    #[display(fmt = "[SquadovError] Internal Error: {}", _0)]
    InternalError(String),
}

impl error::ResponseError for SquadOvError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).finish()
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            SquadOvError::Credentials => StatusCode::UNAUTHORIZED,
            SquadOvError::Unauthorized => StatusCode::UNAUTHORIZED,
            SquadOvError::BadRequest => StatusCode::BAD_REQUEST,
            SquadOvError::NotFound => StatusCode::NOT_FOUND,
            SquadOvError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for SquadOvError {
    fn from(err: sqlx::Error) -> Self {
        return Self::InternalError(format!("Database Error {}", err))
    }
}

impl From<serde_json::Error> for SquadOvError {
    fn from(err: serde_json::Error) -> Self {
        return Self::InternalError(format!("Parse JSON Error {}", err))
    }
}

impl From<url::ParseError> for SquadOvError {
    fn from(err: url::ParseError) -> Self {
        return Self::InternalError(format!("Parse URL Error {}", err))
    }
}
impl From<str::Utf8Error> for SquadOvError {
    fn from(err: str::Utf8Error) -> Self {
        return Self::InternalError(format!("String from UTF-8 Bytes Error {}", err))
    }
}

impl From<uuid::Error> for SquadOvError {
    fn from(err: uuid::Error) -> Self {
        return Self::InternalError(format!("Parse UUID Error {}", err))
    }
}

impl From<base64::DecodeError> for SquadOvError {
    fn from(err: base64::DecodeError) -> Self {
        return Self::InternalError(format!("Base64 Decode Error {}", err))
    }
}

impl From<io::Error> for SquadOvError {
    fn from(err: io::Error) -> Self {
        return Self::InternalError(format!("IO Error {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for SquadOvError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        return Self::InternalError(format!("JWT Error {}", err))
    }
}

impl From<reqwest::Error> for SquadOvError {
    fn from(err: reqwest::Error) -> Self {
        return Self::InternalError(format!("HTTP Request Error {}", err))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for SquadOvError {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        return Self::InternalError(format!("HTTP Header Value Error {}", err))
    }
}

impl From<openssl::error::ErrorStack> for SquadOvError {
    fn from(err: openssl::error::ErrorStack) -> Self {
        return Self::InternalError(format!("OpenSSL Error {}", err))
    }
}

impl From<std::num::ParseIntError> for SquadOvError {
    fn from(err: std::num::ParseIntError) -> Self {
        return Self::InternalError(format!("Parse Int {}", err))
    }
}

impl From<std::num::ParseFloatError> for SquadOvError {
    fn from(err: std::num::ParseFloatError) -> Self {
        return Self::InternalError(format!("Parse Float {}", err))
    }
}

impl<T : num_enum::TryFromPrimitive> From<num_enum::TryFromPrimitiveError<T>> for SquadOvError {
    fn from(err: num_enum::TryFromPrimitiveError<T>) -> Self {
        return Self::InternalError(format!("TryFromPrimitive {}", err))
    }
}

impl<T> From<std::sync::PoisonError<T>> for SquadOvError {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        return Self::InternalError(format!("Sync PoisonError {}", err))
    }
}

impl From<actix_web::error::PayloadError> for SquadOvError {
    fn from(err: actix_web::error::PayloadError) -> Self {
        return Self::InternalError(format!("Payload {}", err))
    }
}