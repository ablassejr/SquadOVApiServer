use actix_web::{error, HttpResponse, http::StatusCode, dev::HttpResponseBuilder};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum AuthError {
    #[display(fmt = "[AuthError] Invalid credentials.")]
    Credentials,
    #[display(fmt = "[AuthError] Unauthorized/not logged in.")]
    Unauthorized,
    #[display(fmt = "[AuthError] Internal system error: {}", message)]
    System { message: String },
    #[display(fmt = "[AuthError] Invalid Request")]
    BadRequest
}

impl error::ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).finish()
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            AuthError::Credentials => StatusCode::UNAUTHORIZED,
            AuthError::Unauthorized => StatusCode::UNAUTHORIZED,
            AuthError::System{..} => StatusCode::INTERNAL_SERVER_ERROR,
            AuthError::BadRequest => StatusCode::BAD_REQUEST,
        }
    }
}