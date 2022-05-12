use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use squadov_common::{
    SquadOvError,
    combatlog::db,
};
use crate::{
    api::{
        ApiApplication,
        auth::SquadOVSession,
    },
};
use std::sync::Arc;
use serde::Deserialize;

pub async fn get_combatlog_config_handler(app : web::Data<Arc<ApiApplication>>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        &app.config.combatlog.hostname
    ))
}

#[derive(Deserialize)]
pub struct CombatLogPath {
    partition_key: String,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
pub struct CombatLogData {
    cl_state: serde_json::Value,
}

pub async fn create_update_combat_log_handler(app : web::Data<Arc<ApiApplication>>, path: web::Path<CombatLogPath>, data: web::Json<CombatLogData>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    db::create_combat_log(&*app.pool, &path.partition_key, session.user.id, data.into_inner().cl_state).await?;
    Ok(HttpResponse::NoContent().finish())
}