use actix_web::{web, HttpResponse, HttpRequest};
use crate::{
    api,
    api::{
        v1::CommunityPathInput,
        auth::SquadOVSession,
    },
};
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    community::{
        db,
        CommunityListQuery,
    },
};
use serde_qs::actix::QsQuery;

pub async fn get_community_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CommunityPathInput>) -> Result<HttpResponse, SquadOvError> {
    Ok(
        HttpResponse::Ok().json(
            db::get_community_from_id(&*app.pool, path.community_id).await?
        )
    )
}

pub async fn list_communities_handler(app : web::Data<Arc<api::ApiApplication>>, filter: QsQuery<CommunityListQuery>, request: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let communities = if filter.only_me {
        let extensions = request.extensions();
        let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::BadRequest)?;
        db::find_communities_for_user(&*app.pool, session.user.id).await?
    } else {
        vec![]
    };

    Ok(HttpResponse::Ok().json(communities))
}