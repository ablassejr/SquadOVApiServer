use actix_web::{web, HttpResponse};
use crate::{
    api,
    api::v1::CommunityPathInput
};
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    community::{
        db,
    },
};
use serde_qs::actix::QsQuery;
use serde::Deserialize;

pub async fn get_community_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CommunityPathInput>) -> Result<HttpResponse, SquadOvError> {
    Ok(
        HttpResponse::Ok().json(
            db::get_community_from_id(&*app.pool, path.community_id).await?
        )
    )
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct CommunityListQuery {
    #[serde(default)]
    onlyMe: bool,
}

pub async fn list_communities_handler(app : web::Data<Arc<api::ApiApplication>>, filter: QsQuery<CommunityListQuery>) -> Result<HttpResponse, SquadOvError> {
    Err(SquadOvError::NotFound)
}