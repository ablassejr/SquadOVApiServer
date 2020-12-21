use squadov_common::{
    SquadOvError,
    WoWCombatLogState
};
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Executor, Postgres};

impl api::ApiApplication {
    async fn create_wow_combat_log<'a, T>(&self, ex: T, user_id: i64, state: &WoWCombatLogState) -> Result<Uuid, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        let uuid = Uuid::new_v4();
        sqlx::query!(
            "
            INSERT INTO squadov.wow_combat_logs (
                uuid,
                user_id,
                combat_log_version,
                advanced_log,
                build_version
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5
            )
            ",
            &uuid,
            user_id,
            &state.combat_log_version,
            state.advanced_log,
            &state.build_version
        )
            .fetch_one(ex)
            .await?;
        Ok(uuid)
    }
}

pub async fn create_wow_combat_log_handler(app : web::Data<Arc<api::ApiApplication>>, state: web::Json<WoWCombatLogState>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_combat_log(&mut tx, session.user.id, &state).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}