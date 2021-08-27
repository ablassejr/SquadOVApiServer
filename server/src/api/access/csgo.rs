use actix_web::{HttpRequest};
use squadov_common::{
    SquadOvError,
    csgo
};
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

pub struct CsgoMatchUserMatchupBasicData {
    pub match_uuid: Uuid,
    pub user_id: i64
}

pub struct CsgoMatchUserPathObtainer {
    pub match_uuid_key: &'static str,
    pub user_id_key: &'static str
}

pub struct CsgoMatchUserMatchupChecker {
    pub obtainer: CsgoMatchUserPathObtainer
}

#[async_trait]
impl super::AccessChecker<CsgoMatchUserMatchupBasicData> for CsgoMatchUserMatchupChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<CsgoMatchUserMatchupBasicData, SquadOvError> {
        Ok(CsgoMatchUserMatchupBasicData{
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

    async fn check(&self, app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, data: CsgoMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(csgo::is_user_in_csgo_match(&*app.pool, data.user_id, &data.match_uuid).await?)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: CsgoMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}