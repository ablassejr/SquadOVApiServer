use crate::api::{
    ApiApplication,
    auth::SquadOVSession,
};
use actix_web::{web, HttpResponse, HttpRequest};
use squadov_common::{
    SquadOvError,
    profile,
};
use serde::Deserialize;
use serde_qs::actix::QsQuery;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct UserProfileQuery {
    id: Option<i64>,
    slug: Option<String>,
}

pub async fn get_basic_profile_handler(app : web::Data<Arc<ApiApplication>>, query: QsQuery<UserProfileQuery>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    // Need to determine WHO is making this request.
    // Is it public? Or is there an actual session behind this.
    let extensions = req.extensions();
    let request_user_id = extensions.get::<SquadOVSession>().map(|x| { x.user.id });

    let raw_profile = if let Some(id) = query.id {
        profile::get_user_profile_from_id(&*app.pool, id).await?
    } else if let Some(slug) = &query.slug {
        profile::get_user_profile_from_slug(&*app.pool, &slug).await?
    } else {
        return Err(SquadOvError::BadRequest);
    };

    Ok(HttpResponse::Ok().json(
        profile::get_user_profile_basic_serialized_with_requester(&*app.pool, raw_profile, request_user_id).await?
    ))
}