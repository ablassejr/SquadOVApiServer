use actix_web::{HttpRequest};
use squadov_common::{
    SquadOvError,
    hearthstone
};
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

pub struct HearthstoneMatchUserMatchupBasicData {
    pub match_uuid: Uuid,
    pub user_id: i64
}

pub struct HearthstoneMatchUserPathObtainer {
    pub match_uuid_key: &'static str,
    pub user_id_key: &'static str
}

pub struct HearthstoneMatchUserMatchupChecker {
    pub obtainer: HearthstoneMatchUserPathObtainer
}

#[async_trait]
impl super::AccessChecker<HearthstoneMatchUserMatchupBasicData> for HearthstoneMatchUserMatchupChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<HearthstoneMatchUserMatchupBasicData, SquadOvError> {
        Ok(HearthstoneMatchUserMatchupBasicData{
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

    async fn check(&self, app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, data: HearthstoneMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(hearthstone::is_user_in_hearthstone_match(&*app.pool, data.user_id, &data.match_uuid).await?)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: HearthstoneMatchUserMatchupBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}