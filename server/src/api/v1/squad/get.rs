use actix_web::{web, HttpResponse};
use crate::api;
use crate::api::v1::UserResourcePath;
use std::sync::Arc;
use squadov_common::{SquadOvError, SquadOvSquad, SquadRole, SquadOvSquadMembership};

impl api::ApiApplication {
    async fn get_squad(&self, squad_id: i64) -> Result<SquadOvSquad, SquadOvError> {
        let squad = sqlx::query_as!(
            SquadOvSquad,
            r#"
            SELECT
                sq.id AS "id!",
                sq.squad_name AS "squad_name!",
                sq.squad_group AS "squad_group!",
                sq.creation_time AS "creation_time!",
                sq.member_count AS "member_count!"
            FROM squadov.squad_overview AS sq
            WHERE id = $1
            "#,
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

    pub async fn get_squad_user_role(&self, squad_id: i64, user_id: i64) -> Result<Option<SquadRole>, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT squad_role
            FROM squadov.squad_role_assignments
            WHERE squad_id = $1 AND user_id = $2
            "
        )
            .bind(squad_id)
            .bind(user_id)
            .fetch_optional(&*self.pool)
            .await?)
    }

    pub async fn get_user_squads(&self, user_id: i64) -> Result<Vec<SquadOvSquadMembership>, SquadOvError> {
        let raw = sqlx::query!(
            r#"
            SELECT
                sq.id AS "id!",
                sq.squad_name AS "squad_name!",
                sq.squad_group AS "squad_group!",
                sq.creation_time AS "creation_time!",
                sq.member_count AS "member_count!",
                sra.squad_role AS "squad_role: SquadRole"
            FROM squadov.squad_overview AS sq
            INNER JOIN squadov.squad_role_assignments AS sra
                ON sra.squad_id = sq.id
            WHERE sra.user_id = $1
            ORDER BY sq.squad_name
            "#,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?;
        Ok(raw.into_iter().map(|x| {
            SquadOvSquadMembership{
                squad: SquadOvSquad{
                    id: x.id,
                    squad_name: x.squad_name,
                    squad_group: x.squad_group,
                    creation_time: x.creation_time,
                    member_count: x.member_count,
                },
                role: x.squad_role
            }
        }).collect())
    }
}

pub async fn get_squad_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let squad = app.get_squad(path.squad_id).await?;
    Ok(HttpResponse::Ok().json(&squad))
}

pub async fn get_user_squads_handler(app : web::Data<Arc<api::ApiApplication>>, path : web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let squads = app.get_user_squads(path.user_id).await?;
    Ok(HttpResponse::Ok().json(&squads))
}