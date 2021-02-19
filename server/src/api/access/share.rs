use crate::api::{
    ApiApplication,
    auth::SquadOVSession,
};
use squadov_common::SquadOvError;
use std::sync::Arc;
use actix_web::{
    dev::{Path, Url},
    HttpRequest,
};
use async_trait::async_trait;
use uuid::Uuid;

pub struct ShareTokenMetadata {
    path: Path<Url>,
    method: String,
}

pub struct ShareTokenAccessRestricter {}

#[async_trait]
impl super::AccessChecker<ShareTokenMetadata> for ShareTokenAccessRestricter {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<ShareTokenMetadata, SquadOvError> {
        Ok(ShareTokenMetadata{
            path: req.match_info().clone(),
            method: String::from(req.method().as_str()),
        })
    }

    async fn check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: ShareTokenMetadata) -> Result<bool, SquadOvError> {
        Ok(true)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, session: &SquadOVSession, data: ShareTokenMetadata) -> Result<bool, SquadOvError> {
        // If the session doesn't have a share token then this checker isn't relevant.
        if session.share_token.is_none() {
            return Ok(true);
        }

        // First off, reject any HTTP methods that aren't a GET because a share token shouldn't be used for making changes.
        if data.method != "GET" {
            return Ok(false);
        }

        let share_token = session.share_token.as_ref().unwrap();

        // There's three accesses we want to check:
        //  1) Match
        //  2) VOD
        //  3) User
        // In certain cases, there may not be a user so we can ignore it. If it does exist though, it must pass.
        let mut granted_access = false;

        // Next, check for the presence of either a match_uuid or a video_uuid in the path because those are currently the
        // only things that can be shared right now. Then check that this access token grants access to that particular
        // match or video. Note that a specific route can also disable access to a shared token if
        // need be with a separate middleware object.
        if let Some(match_uuid) = data.path.get("match_uuid") {
            if share_token.match_uuid.is_none() || &match_uuid.parse::<Uuid>()? != share_token.match_uuid.as_ref().unwrap()  {
                return Ok(false);
            }
            granted_access = true;
        }
        
        if let Some(video_uuid) = data.path.get("video_uuid") {
            if share_token.video_uuid.is_none() || &video_uuid.parse::<Uuid>()? != share_token.video_uuid.as_ref().unwrap() {
                return Ok(false);
            }
            granted_access = true;
        }

        if let Some(clip_uuid) = data.path.get("clip_uuid") {
            if share_token.clip_uuid.is_none() || &clip_uuid.parse::<Uuid>()? != share_token.clip_uuid.as_ref().unwrap() {
                return Ok(false);
            }
            granted_access = true;
        }

        if let Some(user_id) = data.path.get("user_id") {
            let user_id = user_id.parse::<i64>()?;
            if user_id != session.user.id {
                return Ok(false);
            }
            // Do not grant access purely based on user. It's only used to do additional denying.
            // This way we don't grant access to match listings and stuff like that.
        }

        Ok(granted_access)
    }
}

pub struct DenyShareTokenAccess {}

#[async_trait]
impl super::AccessChecker<()> for DenyShareTokenAccess {
    fn generate_aux_metadata(&self, _req: &HttpRequest) -> Result<(), SquadOvError> {
        Ok(())
    }

    async fn check(&self, _app: Arc<ApiApplication>, session: &SquadOVSession, _data: ()) -> Result<bool, SquadOvError> {
        Ok(session.share_token.is_none())
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: ()) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}