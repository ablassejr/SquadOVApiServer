use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use squadov_common::SquadRole;
use async_trait::async_trait;
use super::UserIdSetObtainer;
use std::collections::HashSet;
use std::iter::FromIterator;

pub trait SquadIdSetObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<i64, squadov_common::SquadOvError>;
}

pub struct SquadIdPathSetObtainer {
    pub key: &'static str
}

impl SquadIdSetObtainer for SquadIdPathSetObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<i64, squadov_common::SquadOvError> {
        let squad_id = match req.match_info().get(self.key) {
            Some(x) => x,
            None => return Err(squadov_common::SquadOvError::BadRequest),
        };

        match squad_id.parse::<i64>() {
            Ok(x) => Ok(x),
            Err(_) => Err(squadov_common::SquadOvError::BadRequest),
        }
    }
}

pub struct SquadAccessBasicData {
    squad_id: i64
}

pub struct SquadAccessChecker<T> {
    // Whether or not this endpoint requires the user
    // in question to be an owner. Using this checker
    // already assumes that we want to check that the user
    // is a member.
    pub requires_owner: bool,
    pub obtainer: T
}

#[async_trait]
impl<T: SquadIdSetObtainer + Send + Sync> super::AccessChecker<SquadAccessBasicData> for SquadAccessChecker<T> {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<SquadAccessBasicData, squadov_common::SquadOvError> {
        Ok(SquadAccessBasicData{
            squad_id: self.obtainer.obtain(req)?
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: Option<&SquadOVSession>, data: SquadAccessBasicData) -> Result<bool, squadov_common::SquadOvError> {
        let session = session.unwrap();
        let role = app.get_squad_user_role(data.squad_id, session.user.id).await?;
        if role.is_none() {
            return Ok(false);
        }

        let role = role.unwrap();
        if self.requires_owner {
            Ok(role == SquadRole::Owner)
        } else {
            Ok(true)
        }
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: SquadAccessBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}

pub struct SameSquadAccessChecker<T> {
    pub obtainer: T
}

#[async_trait]
impl super::AccessChecker<super::UserAccessSetBasicData> for SameSquadAccessChecker<super::UserIdPathSetObtainer> {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<super::UserAccessSetBasicData, squadov_common::SquadOvError> {
        Ok(super::UserAccessSetBasicData{
            access_set: self.obtainer.obtain(req)?
        })
    }

    async fn check(&self, app: Arc<ApiApplication>, session: Option<&SquadOVSession>, data: super::UserAccessSetBasicData) -> Result<bool, squadov_common::SquadOvError> {
        let session = session.unwrap();
        let user_ids: Vec<i64> = data.access_set.into_iter().collect();
        let mut same_squad_user_ids: HashSet<i64> = HashSet::from_iter(app.get_user_ids_in_same_squad_as_users(&user_ids, None).await?.into_iter());

        // Need to reinsert the user_ids into the HashSet just in case the user isn't in a squad in which
        // case their user id won't be returned by the get_user_ids_in_same_squad_as_users function.
        same_squad_user_ids.extend(&user_ids);

        Ok(same_squad_user_ids.contains(&session.user.id))
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: Option<&SquadOVSession>, _data: super::UserAccessSetBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}