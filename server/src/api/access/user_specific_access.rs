use actix_web::{HttpRequest};
use squadov_common;
use std::collections::HashSet;
use crate::api::auth::SquadOVSession;

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

/// An access checker to check that the given user's ID is in
/// some set of user IDs as obtained by the object that implements the trait T : UserIdSetObtainer.
pub struct UserSpecificAccessChecker<T : UserIdSetObtainer> {
    pub obtainer: T
}

impl<T: UserIdSetObtainer> super::AccessChecker for UserSpecificAccessChecker<T> {
    fn check(&self, session: &SquadOVSession, req: &HttpRequest) -> Result<bool, squadov_common::SquadOvError> {
        let access_set = self.obtainer.obtain(req)?;
        return Ok(access_set.contains(&session.user.id));
    }
}