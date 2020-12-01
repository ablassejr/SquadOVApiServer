use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{SquadOvError, SquadOvSquad};

impl api::ApiApplication {
    async fn get_squad(&self, squad_id: i64) -> Result<SquadOvSquad, SquadOvError> {
        let squad = sqlx::query_as!(
            SquadOvSquad,
            "
            SELECT *
            FROM squadov.squads
            WHERE id = $1
            ",
            squad_id,
        )
            .fetch_optional(&*self.pool)
            .await?;

        if squad.is_none() {
            Err(SquadOvError::NotFound)
        } else {
            Ok(squad.unwrap())
        }
    }
}

pub async fn get_squad_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let squad = app.get_squad(path.squad_id).await?;
    Ok(HttpResponse::Ok().json(&squad))
}