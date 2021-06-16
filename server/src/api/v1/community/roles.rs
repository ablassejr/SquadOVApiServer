use actix_web::{web, HttpResponse};
use crate::{
    api::{
        v1::{
            CommunityPathInput,
            CommunityRolePathInput,
        },
        ApiApplication,
    },
};
use std::sync::Arc;
use squadov_common::{
    SquadOvError,
    community::{
        roles,
        CommunityRole,
    },
};

pub async fn list_roles_in_community_handler(app : web::Data<Arc<ApiApplication>>, path: web::Path<CommunityPathInput>) -> Result<HttpResponse, SquadOvError> {
    Ok(
        HttpResponse::Ok().json(
            roles::list_community_roles(&*app.pool, path.community_id).await?
        )
    )
}

pub async fn remove_role_from_community_handler(app : web::Data<Arc<ApiApplication>>, path: web::Path<CommunityRolePathInput>) -> Result<HttpResponse, SquadOvError> {
    roles::delete_community_role(&*app.pool, path.community_id, path.role_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn edit_role_in_community_handler(app : web::Data<Arc<ApiApplication>>, path: web::Path<CommunityRolePathInput>, data: web::Json<CommunityRole>) -> Result<HttpResponse, SquadOvError> {
    let mut role = data.into_inner();
    role.community_id = path.community_id;
    role.id = path.role_id;

    roles::edit_community_role(&*app.pool, &role).await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn create_role_in_community_handler(app : web::Data<Arc<ApiApplication>>, path: web::Path<CommunityRolePathInput>, data: web::Json<CommunityRole>) -> Result<HttpResponse, SquadOvError> {
    let mut role = data.into_inner();
    role.community_id = path.community_id;
    role.id = path.role_id;
    Ok(HttpResponse::Ok().json(
        roles::create_community_role(&*app.pool, &role).await?
    ))
}