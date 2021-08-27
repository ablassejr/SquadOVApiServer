use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashSet;

pub struct NullUserSetAccessChecker {
}

#[async_trait]
impl super::AccessChecker<super::UserAccessSetBasicData> for NullUserSetAccessChecker {
    fn generate_aux_metadata(&self, _req: &HttpRequest) -> Result<super::UserAccessSetBasicData, SquadOvError> {
        Ok(super::UserAccessSetBasicData{
            access_set: HashSet::new(),
        })
    }

    async fn check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: super::UserAccessSetBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: super::UserAccessSetBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}