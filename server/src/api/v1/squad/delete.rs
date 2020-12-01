use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use std::sync::Arc;
use squadov_common::{SquadOvError, SquadRole};
use crate::api::auth::SquadOVSession;
use sqlx::{Transaction, Postgres};

impl api::ApiApplication {
    async fn delete_squad(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            DELETE FROM squadov.squads
            WHERE id = $1
            ",
            squad_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    async fn leave_squad(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            DELETE FROM squadov.squad_role_assignments
            WHERE squad_id = $1 AND user_id = $2
            ",
            squad_id,
            user_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }
}

pub async fn delete_squad_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.delete_squad(&mut tx, data.squad_id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn leave_squad_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Path<super::SquadSelectionInput>, request: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::BadRequest)
    };

    // Ensure that the owner user isn't trying to leave. Is there a way to
    // get this to be represented as a PostgreSQL constraint so that DB
    // operation just fails instead?
    let role = app.get_squad_user_role(data.squad_id, session.user.id).await?;
    if role == SquadRole::Owner {
        return Err(SquadOvError::BadRequest);
    }

    let mut tx = app.pool.begin().await?;
    app.leave_squad(&mut tx, data.squad_id, session.user.id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}