use actix_web::{error, HttpResponse, http::StatusCode, dev::HttpResponseBuilder};
use derive_more::{Display};
use sqlx;
use url;
use std::str;
use std::io;
use jsonwebtoken;
use reqwest;
use openssl;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    #[serde(rename="duplicateFlag")]
    duplicate_flag: bool
}

#[derive(Debug, Display, PartialEq)]
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
    #[display(fmt = "[SquadovError] Duplicate Error")]
    Duplicate,
    #[display(fmt = "[SquadovError] Defer: {}", _0)]
    Defer(i64),
    #[display(fmt = "[SquadovError] Rate Limit")]
    RateLimit,
}

impl error::ResponseError for SquadOvError {
    fn error_response(&self) -> HttpResponse {
        let body = ErrorBody{
            duplicate_flag: *self == SquadOvError::Duplicate
        };

        HttpResponseBuilder::new(self.status_code()).json(&body)
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            SquadOvError::Credentials => StatusCode::UNAUTHORIZED,
            SquadOvError::Unauthorized => StatusCode::UNAUTHORIZED,
            SquadOvError::BadRequest => StatusCode::BAD_REQUEST,
            SquadOvError::NotFound => StatusCode::NOT_FOUND,
            SquadOvError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SquadOvError::Duplicate => StatusCode::BAD_REQUEST,
            SquadOvError::Defer(_) => StatusCode::SERVICE_UNAVAILABLE,
            SquadOvError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

impl From<sqlx::Error> for SquadOvError {
    fn from(err: sqlx::Error) -> Self {
        let generic_error = Self::InternalError(format!("Database Error {}", err));
        match err {
            sqlx::Error::Database(db_err) => {
                let code = db_err.code();
                // Check for the Postgres duplicate key violates unique constraint error
                // so that we can return a "Duplicate" error instead so that we can give
                // more context to the user.
                if code.is_some() {
                    let code = code.unwrap();
                    if code == "23505" || code == "23P01" {
                        Self::Duplicate
                    } else {
                        generic_error
                    }
                } else {
                    generic_error
                }
            },
            _ => generic_error
        }
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
        return Self::InternalError(format!("OpenSSL Error {:?}", err))
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

impl From<prost::EncodeError> for SquadOvError {
    fn from(err: prost::EncodeError) -> Self {
        return Self::InternalError(format!("Proto Encode {}", err))
    }
}

impl From<std::env::VarError> for SquadOvError {
    fn from(err: std::env::VarError) -> Self {
        return Self::InternalError(format!("Env Var {}", err))
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for SquadOvError {
    fn from(err: std::sync::mpsc::SendError<T>) -> Self {
        return Self::InternalError(format!("MPSC Send Error {}", err))
    }
}

impl From<actix_multipart::MultipartError> for SquadOvError {
    fn from(err: actix_multipart::MultipartError) -> Self {
        return Self::InternalError(format!("Multipart Error {}", err))
    }
}

impl From<reqwest::header::ToStrError> for SquadOvError {
    fn from(err: reqwest::header::ToStrError) -> Self {
        return Self::InternalError(format!("To Str {}", err))
    }
}

impl From<std::array::TryFromSliceError> for SquadOvError {
    fn from(err: std::array::TryFromSliceError) -> Self {
        return Self::InternalError(format!("Slice into Array {}", err))
    }
}

impl From<lapin::Error> for SquadOvError {
    fn from(err: lapin::Error) -> Self {
        return Self::InternalError(format!("AMQP {}", err))
    }
}

impl From<actix_web::Error> for SquadOvError {
    fn from(err: actix_web::Error) -> Self {
        return Self::InternalError(format!("Actix Web {}", err))
    }
}

impl From<hex::FromHexError> for SquadOvError {
    fn from(err: hex::FromHexError) -> Self {
        return Self::InternalError(format!("Hex Decode {}", err))
    }
}