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
        SquadOvCommunity,
    },
};

pub async fn delete_community_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CommunityPathInput>) -> Result<HttpResponse, SquadOvError> {
    db::delete_community(&*app.pool, path.community_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn edit_community_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<CommunityPathInput>, data: web::Json<SquadOvCommunity>) -> Result<HttpResponse, SquadOvError> {
    let mut community = data.into_inner();
    community.id = path.community_id;

    // Question, if the user decides to switch the community to require subs does that mean
    // the previous users should be removed? Probably not - grandfathering in old users probably
    // makes the most sense and is also just easier to do :)
    let mut tx = app.pool.begin().await?;
    db::edit_community(&mut tx, &community).await?;
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}