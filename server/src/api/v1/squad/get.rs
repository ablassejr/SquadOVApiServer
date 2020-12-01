use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{SquadOvError, SquadOvSquad, SquadRole};

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

    pub async fn get_squad_user_role(&self, squad_id: i64, user_id: i64) -> Result<SquadRole, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT squad_role
            FROM squadov.squad_role_assignments
            WHERE squad_id = $1 AND user_id = $2
            "
        )
            .bind(squad_id)
            .bind(user_id)
            .fetch_one(&*self.pool)
            .await?)
    }
}

pub async fn get_squad_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let squad = app.get_squad(path.squad_id).await?;
    Ok(HttpResponse::Ok().json(&squad))
}