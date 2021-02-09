use actix_web::{HttpRequest};
use squadov_common::SquadOvError;
use std::collections::HashSet;
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;

/// This trait is used by the UserSpecificAccessChecker to obtain a set of users
/// that should be granted to a particular path/resource.
pub trait UserIdSetObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<HashSet<i64>, squadov_common::SquadOvError>;
}

pub struct UserIdPathSetObtainer {
    pub key: &'static str
}

impl UserIdSetObtainer for UserIdPathSetObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<HashSet<i64>, squadov_common::SquadOvError> {
        let user_id = match req.match_info().get(self.key) {
            Some(x) => x,
            None => return Err(squadov_common::SquadOvError::BadRequest),
        };

        let user_id = match user_id.parse::<i64>() {
            Ok(x) => x,
            Err(_) => return Err(squadov_common::SquadOvError::BadRequest),
        };

        return Ok([user_id].iter().cloned().collect());
    }
}

pub struct UserAccessSetBasicData {
    pub access_set: HashSet<i64>
}

/// An access checker to check that the given user's ID is in
/// some set of user IDs as obtained by the object that implements the trait T : UserIdSetObtainer.
pub struct UserSpecificAccessChecker<T> {
    pub obtainer: T
}

#[async_trait]
impl super::AccessChecker<UserAccessSetBasicData> for UserSpecificAccessChecker<UserIdPathSetObtainer> {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<UserAccessSetBasicData, squadov_common::SquadOvError> {
        Ok(UserAccessSetBasicData{
            access_set: self.obtainer.obtain(req)?
        })
    }

    async fn check(&self, _app: Arc<ApiApplication>, session: &SquadOVSession, data: UserAccessSetBasicData) -> Result<bool, squadov_common::SquadOvError> {
        return Ok(data.access_set.contains(&session.user.id));
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: UserAccessSetBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}

pub struct AdminAccessChecker {
}

#[async_trait]
impl super::AccessChecker<()> for AdminAccessChecker {
    fn generate_aux_metadata(&self, _req: &HttpRequest) -> Result<(), squadov_common::SquadOvError> {
        Ok(())
    }

    async fn check(&self, _app: Arc<ApiApplication>, session: &SquadOVSession, _data: ()) -> Result<bool, squadov_common::SquadOvError> {
        Ok(session.user.is_admin)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: ()) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}