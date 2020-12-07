mod valorant;

pub use valorant::*;

use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;

pub struct RiotAccountAccessBasicData {
    pub user_id: i64,
    pub puuid: String
}

pub struct RiotAccountPathObtainer {
    pub user_id_key: &'static str,
    pub puuid_key: &'static str
}

pub struct RiotAccountAccessChecker {
    pub obtainer: RiotAccountPathObtainer
}

#[async_trait]
impl super::AccessChecker<RiotAccountAccessBasicData> for RiotAccountAccessChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<RiotAccountAccessBasicData, SquadOvError> {
        Ok(RiotAccountAccessBasicData{
            user_id: match req.match_info().get(self.obtainer.user_id_key) {
                Some(x) => x.parse::<i64>()?,
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
            puuid: match req.match_info().get(self.obtainer.puuid_key) {
                Some(x) => String::from(x),
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },  
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, _session: &SquadOVSession, data: RiotAccountAccessBasicData) -> Result<bool, SquadOvError> {
        match app.get_riot_account(data.user_id, &data.puuid).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false)
        }
    }
}