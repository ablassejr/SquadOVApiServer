use actix_web::{HttpRequest};
use squadov_common::{
    access::check_user_has_access_to_match_vod_from_user,
    SquadOvError
};
use crate::api::auth::SquadOVSession;
use crate::api::ApiApplication;
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

pub struct MatchVodBasicData {
    pub match_uuid: Option<Uuid>,
    pub video_uuid: Option<Uuid>,
    pub user_id: Option<i64>,
}

pub struct MatchVodAccessChecker<T> {
    pub obtainer: T
}

pub struct MatchVodPathObtainer {
    pub match_key: Option<&'static str>,
    pub video_key: Option<&'static str>,
    pub user_key: Option<&'static str>,
}

impl MatchVodPathObtainer {
    fn obtain(&self, req: &HttpRequest) -> Result<MatchVodBasicData, SquadOvError> {
        let mut ret = MatchVodBasicData{
            match_uuid: None,
            video_uuid: None,
            user_id: None,
        };

        if let Some(match_key) = self.match_key {
            ret.match_uuid = req.match_info().get(match_key).map(|x| {
                Ok(Uuid::parse_str(x)?)
            }).map_or(Ok(None), |x: Result<Uuid, SquadOvError>| x.map(Some))?;
        }

        if let Some(video_key) = self.video_key {
            ret.video_uuid = req.match_info().get(video_key).map(|x| {
                Ok(Uuid::parse_str(x)?)
            }).map_or(Ok(None), |x: Result<Uuid, SquadOvError>| x.map(Some))?;
        }

        if let Some(user_key) = self.user_key {
            ret.user_id = req.match_info().get(user_key).map(|x| {
                Ok(x.parse::<i64>()?)
            }).map_or(Ok(None), |x: Result<i64, SquadOvError>| x.map(Some))?;
        }

        Ok(ret)
    }
}

#[async_trait]
impl super::AccessChecker<MatchVodBasicData> for MatchVodAccessChecker<MatchVodPathObtainer> {
    fn generate_aux_metadata(&self, req: &HttpRequest) -> Result<MatchVodBasicData, SquadOvError> {
        Ok(self.obtainer.obtain(req)?)
    }

    async fn check(&self, app: Arc<ApiApplication>, session: &SquadOVSession, data: MatchVodBasicData) -> Result<bool, SquadOvError> {
        Ok(check_user_has_access_to_match_vod_from_user(
            &*app.pool,
            session.user.id,
            if data.user_id.is_some() {
                data.user_id
            } else {
                if let Some(video_uuid) = data.video_uuid {
                    Some(app.get_vod_owner_user_id(&video_uuid).await?)
                } else {
                    None
                }
            },
            data.match_uuid,
            data.video_uuid,
        ).await?)
    }

    async fn post_check(&self, _app: Arc<ApiApplication>, _session: &SquadOVSession, _data: MatchVodBasicData) -> Result<bool, SquadOvError> {
        Ok(true)
    }
}