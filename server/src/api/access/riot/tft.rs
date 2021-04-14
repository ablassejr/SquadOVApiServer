use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use crate::api::access::AccessChecker;
use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashSet;
use std::iter::FromIterator;
use uuid::Uuid;

pub struct TftMatchUuidData {
    pub match_uuid: Uuid
}

pub struct TftMatchUuidPathObtainer {
    pub match_uuid_key: &'static str
}

pub struct TftMatchAccessChecker {
    pub obtainer: TftMatchUuidPathObtainer
}

#[async_trait]
impl AccessChecker<TftMatchUuidData> for TftMatchAccessChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<TftMatchUuidData, SquadOvError> {
        Ok(TftMatchUuidData{
            match_uuid: match req.match_info().get(self.obtainer.match_uuid_key) {
                Some(x) => x.parse()?,
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: TftMatchUuidData) -> Result<bool, SquadOvError> {
        // The user must be either in the match or a squad member of a user in that match.        
        let access_set: HashSet<i64> = HashSet::from_iter(app.list_squadov_accounts_can_access_tft_match(&data.match_uuid).await?);
        Ok(access_set.contains(&session.user.id))
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: TftMatchUuidData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}