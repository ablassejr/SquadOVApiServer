use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::SquadOvError;
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
}

pub async fn delete_squad_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.delete_squad(&mut tx, data.squad_id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}