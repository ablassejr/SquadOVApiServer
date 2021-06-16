use async_trait::async_trait;
use squadov_common::{
    SquadOvError,
    community::{
        CommunityRole,
        db,
    },
};
use std::sync::Arc;
use crate::{
    api::{
        ApiApplication,
        auth::SquadOVSession,
    },
};
use actix_web::HttpRequest;

pub struct CommunityAccessBasicData {
    community_id: i64,
}

pub struct CommunityIdPathSetObtainer {
    pub key: &'static str
}

impl CommunityIdPathSetObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<i64, SquadOvError> {
        let community_id = req.match_info().get(self.key).ok_or(SquadOvError::BadRequest)?;
        Ok(community_id.parse::<i64>()?)
    }
}

pub struct CommunityAccessChecker {
    pub obtainer: CommunityIdPathSetObtainer,
    pub is_owner: bool,
    pub can_manage: bool,
    pub can_moderate: bool,
    pub can_invite: bool,
    pub can_share: bool,
}

#[async_trait]
impl super::AccessChecker<CommunityAccessBasicData> for CommunityAccessChecker {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<CommunityAccessBasicData, SquadOvError> {
        Ok(CommunityAccessBasicData{
            community_id: self.obtainer.obtain(req)?
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: CommunityAccessBasicData) -> Result<bool, SquadOvError> {
        if self.is_owner{
            let community = db::get_community_from_id(&*app.pool, data.community_id).await?;
            if community.creator_user_id != session.user.id {
                return Ok(false);
            }
        }

        let roles = db::get_user_community_roles(&*app.pool, data.community_id, session.user.id).await?;
        if roles.is_empty() {
            return Ok(false);
        }

        let mut permission_set = CommunityRole{
            id: -1,
            community_id: -1,
            name: String::new(),
            can_manage: false,
            can_moderate: false,
            can_invite: false,
            can_share: false,
            is_default: false,
        };
        for r in roles {
            permission_set.can_manage |= r.can_manage;
            permission_set.can_moderate |= r.can_moderate;
            permission_set.can_invite |= r.can_invite;
            permission_set.can_share |= r.can_share;
        }

        if self.can_manage && !permission_set.can_manage {
            return Ok(false);
        }

        if self.can_moderate && !permission_set.can_moderate {
            return Ok(false);
        }

        if self.can_invite && !permission_set.can_invite {
            return Ok(false);
        }

        if self.can_share && !permission_set.can_share {
            return Ok(false);
        }

        Ok(true)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: CommunityAccessBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}