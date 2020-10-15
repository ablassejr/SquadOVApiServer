use crate::common;
use crate::api;
use actix_web::{web, HttpResponse, HttpRequest};
use uuid::Uuid;
use sqlx::{Transaction, Executor, Postgres};
use serde::Serialize;
use crate::api::auth::SquadOVSession;

#[derive(Serialize)]
struct CreateAimlabTaskResponse<'a> {
    #[serde(rename = "matchUuid")]
    match_uuid: &'a Uuid
}

impl api::ApiApplication {
    pub async fn create_new_aimlab_task(&self, tx : &mut Transaction<'_, Postgres>, uuid: &Uuid, raw_match : super::AimlabTask) -> Result<(), common::SquadOvError> {
        tx.execute(
            sqlx::query!(
                "
                INSERT INTO squadov.aimlab_tasks (
                    id,
                    user_id,
                    klutch_id,
                    match_uuid,
                    task_name,
                    mode,
                    score,
                    version,
                    create_date,
                    raw_data
                )
                VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    $9,
                    $10
                )
                ",
                raw_match.id,
                raw_match.user_id,
                &raw_match.klutch_id,
                uuid,
                &raw_match.task_name,
                raw_match.mode,
                raw_match.score,
                &raw_match.version,
                &raw_match.create_date,
                serde_json::from_str::<serde_json::Value>(&raw_match.raw_data)?
            )
        ).await?;
        return Ok(());
    }
}

pub async fn create_new_aimlab_task_handler(data : web::Json<super::AimlabTask>, app : web::Data<api::ApiApplication>, request : HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let mut raw_data = data.into_inner();

    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(common::SquadOvError::BadRequest)
    };

    raw_data.user_id = session.user.id;

    let mut tx = app.pool.begin().await?;
    // Create a new match ID and then create the match.
    let internal_match = app.create_new_match(&mut tx).await?;
    app.create_new_aimlab_task(&mut tx, &internal_match.uuid, raw_data).await?;
    tx.commit().await?;

    return Ok(HttpResponse::Ok().json(
        &CreateAimlabTaskResponse{
            match_uuid: &internal_match.uuid
        }
    ))
}