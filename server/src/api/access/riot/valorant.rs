use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use crate::api::access::AccessChecker;
use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashSet;
use std::iter::FromIterator;

pub struct ValorantMatchUuidData {
    pub match_id: String
}

pub struct ValorantMatchUuidPathObtainer {
    pub match_id_key: &'static str
}

pub struct ValorantMatchAccessChecker {
    pub obtainer: ValorantMatchUuidPathObtainer
}

#[async_trait]
impl AccessChecker<ValorantMatchUuidData> for ValorantMatchAccessChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<ValorantMatchUuidData, SquadOvError> {
        Ok(ValorantMatchUuidData{
            match_id: match req.match_info().get(self.obtainer.match_id_key) {
                Some(x) => String::from(x),
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: ValorantMatchUuidData) -> Result<bool, SquadOvError> {
        // The user must be either in the match or a squad member of a user in that match.        
        let access_set: HashSet<i64> = HashSet::from_iter(app.list_squadov_accounts_can_access_match(&data.match_id).await?);
        Ok(access_set.contains(&session.user.id))
    }
}