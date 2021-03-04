use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

pub struct WowMatchUserMatchupBasicData {
    pub match_uuid: Uuid,
    pub user_id: i64
}

pub struct WowMatchUserPathObtainer {
    pub match_uuid_key: &'static str,
    pub user_id_key: &'static str
}

pub struct WowMatchUserMatchupChecker {
    pub obtainer: WowMatchUserPathObtainer
}

#[async_trait]
impl super::AccessChecker<WowMatchUserMatchupBasicData> for WowMatchUserMatchupChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<WowMatchUserMatchupBasicData, SquadOvError> {
        Ok(WowMatchUserMatchupBasicData{
            match_uuid: match req.match_info().get(self.obtainer.match_uuid_key) {
                Some(x) => x.parse::<Uuid>()?,
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
            user_id: match req.match_info().get(self.obtainer.user_id_key) {
                Some(x) => x.parse::<i64>()?,
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, _session: &SquadOVSession, data: WowMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        // Check that the given user (in the path) is actually a part of the given match. 
        Ok(app.get_wow_match_view_for_user_match(data.user_id, &data.match_uuid).await?.is_some())
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: WowMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}