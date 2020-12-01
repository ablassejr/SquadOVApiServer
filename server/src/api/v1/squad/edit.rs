use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::SquadOvError;
use sqlx::{Transaction, Postgres};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EditSquadInput {
    #[serde(rename="squadName")]
    squad_name: String
}

impl api::ApiApplication {
    async fn edit_squad(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, squad_name: &str) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.squads
            SET squad_name = $2
            WHERE id = $1
            ",
            squad_id,
            squad_name
        )
            .execute(tx)
            .await?;
        Ok(())
    }
}

pub async fn edit_squad_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>, data: web::Json<EditSquadInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.edit_squad(&mut tx, path.squad_id, &data.squad_name).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}