use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;
use std::collections::HashSet;
use std::iter::FromIterator;

pub struct VodAccessBasicData {
    pub video_uuid: Uuid
}

pub struct VodPathObtainer {
    pub video_uuid_key: &'static str,
}

pub struct VodAccessChecker {
    pub must_be_vod_owner: bool,
    pub obtainer: VodPathObtainer
}

#[async_trait]
impl super::AccessChecker<VodAccessBasicData> for VodAccessChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<VodAccessBasicData, SquadOvError> {
        Ok(VodAccessBasicData{
            video_uuid: match req.match_info().get(self.obtainer.video_uuid_key) {
                Some(x) => x.parse::<Uuid>()?,
                None => return Err(squadov_common::SquadOvError::BadRequest),
            },
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: VodAccessBasicData) -> Result<bool, SquadOvError> {
        // The only users who should be able to access the VOD are those who are in the same squad as the owner of the VOD.
        // Ideally this would just use the SameSquadAccessChecker somehow?
        if self.must_be_vod_owner {
            let user_id = app.get_vod_owner(&data.video_uuid).await?;
            Ok(user_id.unwrap_or(-1) == session.user.id)
        } else {
            let same_squad_user_ids: HashSet<i64> = HashSet::from_iter(app.list_squadov_accounts_can_access_vod(&data.video_uuid).await?.into_iter());
            Ok(same_squad_user_ids.contains(&session.user.id))
        }
    }
}