use squadov_common::{SquadOvError, SerializedUserSession, SessionJwtClaims};
use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use serde::Deserialize;
use chrono::{DateTime, Utc, NaiveDateTime};

#[derive(Deserialize)]
pub struct RefreshSessionInput {
    #[serde(rename="sessionId")]
    session_id: String
}

pub async fn refresh_user_session_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<RefreshSessionInput>) -> Result<HttpResponse, SquadOvError> {
    let session = app.session.get_session_from_id(&data.session_id, &app.pool).await?;
    if session.is_none() {
        return Err(SquadOvError::Unauthorized);
    }
    let session = app.refresh_session_if_necessary(session.unwrap()).await?;

    // Extract expiration from the access token JWT.
    let token = jsonwebtoken::dangerous_insecure_decode::<SessionJwtClaims>(&session.access_token)?;
    Ok(HttpResponse::Ok().json(SerializedUserSession{
        session_id: session.session_id.clone(),
        expiration: DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(token.claims.exp, 0), Utc)
    }))
}