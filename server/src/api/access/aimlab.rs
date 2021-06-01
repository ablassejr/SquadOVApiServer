use actix_web::{HttpRequest};
use squadov_common::{
    SquadOvError,
    aimlab
};
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

pub struct AimlabMatchUserMatchupBasicData {
    pub match_uuid: Uuid,
    pub user_id: i64
}

pub struct AimlabMatchUserPathObtainer {
    pub match_uuid_key: &'static str,
    pub user_id_key: &'static str
}

pub struct AimlabMatchUserMatchupChecker {
    pub obtainer: AimlabMatchUserPathObtainer
}

#[async_trait]
impl super::AccessChecker<AimlabMatchUserMatchupBasicData> for AimlabMatchUserMatchupChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<AimlabMatchUserMatchupBasicData, SquadOvError> {
        Ok(AimlabMatchUserMatchupBasicData{
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

    async fn check(&self, app: Arc<ApiApplication>, _session: &SquadOVSession, data: AimlabMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(aimlab::is_user_aimlab_task_owner(&*app.pool, data.user_id, &data.match_uuid).await?)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: AimlabMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}