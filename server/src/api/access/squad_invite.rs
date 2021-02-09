use actix_web::{HttpRequest};
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use squadov_common::SquadOvError;

pub trait SquadInviteObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<uuid::Uuid, squadov_common::SquadOvError>;
}

pub struct SquadInvitePathObtainer {
    pub key: &'static str
}

impl SquadInviteObtainer for SquadInvitePathObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<uuid::Uuid, squadov_common::SquadOvError> {
        let invite_uuid = match req.match_info().get(self.key) {
            Some(x) => x,
            None => return Err(squadov_common::SquadOvError::BadRequest),
        };

        match invite_uuid.parse::<uuid::Uuid>() {
            Ok(x) => Ok(x),
            Err(_) => Err(squadov_common::SquadOvError::BadRequest),
        }
    }
}

pub struct SquadInviteAccessBasicData {
    invite_uuid: uuid::Uuid
}

#[async_trait]
impl super::AccessChecker<SquadInviteAccessBasicData> for super::UserSpecificAccessChecker<SquadInvitePathObtainer> {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<SquadInviteAccessBasicData, squadov_common::SquadOvError> {
        Ok(SquadInviteAccessBasicData{
            invite_uuid: self.obtainer.obtain(req)?
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: SquadInviteAccessBasicData) -> Result<bool, squadov_common::SquadOvError> {
        let user_id = app.get_squad_invite_user(&data.invite_uuid).await?;
        Ok(user_id == session.user.id)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: SquadInviteAccessBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}