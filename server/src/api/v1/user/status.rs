use actix::{Addr};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use crate::api;
use squadov_common::{
    SquadOvError,
    squad::status::{UserActivitySession, UserActivityStatusTracker},
    session::SessionVerifier,
};
use async_trait::async_trait;
use std::sync::Arc;
use serde::Deserialize;

#[async_trait]
impl SessionVerifier for api::ApiApplication {
    async fn verify_session_id_for_user(&self, user_id: i64, session_id: String) -> Result<bool, SquadOvError> {
        let session = self.session.get_session_from_id(&session_id, &*self.pool).await?;
        if session.is_none() {
            return Ok(false);
        }
        
        let session = session.unwrap();
        Ok(session.user.id == user_id && self.is_session_valid(&session).await?)
    }
}

#[derive(Deserialize)]
pub struct UserStatusInput {
    user_id: i64
}

pub async fn get_user_status_handler(req: HttpRequest, stream: web::Payload, app : web::Data<Arc<api::ApiApplication>>, tracker: web::Data<Addr<UserActivityStatusTracker>>, path: web::Path<UserStatusInput>) -> Result<HttpResponse, SquadOvError> {
    let resp = ws::start(UserActivitySession::new(path.user_id, tracker.get_ref().clone(), app.get_ref().clone()), &req, stream)?;
    Ok(resp)
}