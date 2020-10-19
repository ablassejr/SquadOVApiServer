use crate::common;
use crate::api;
use actix_web::{web, HttpResponse};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn get_aimlab_task_data(&self, match_id : Uuid) -> Result<Option<super::AimlabTask>, common::SquadOvError> {
        let task = sqlx::query_as!(
            super::AimlabTask,
            "
            SELECT *
            FROM squadov.aimlab_tasks
            WHERE match_uuid = $1
            ",
            match_id,
        )
            .fetch_optional(&*self.pool)
            .await?;
        Ok(task)
    }
}

pub async fn get_aimlab_task_data_handler(data : web::Path<super::AimlabTaskGetInput>, app : web::Data<api::ApiApplication>) -> Result<HttpResponse, common::SquadOvError> {
    let task = app.get_aimlab_task_data(data.match_uuid).await?;
    match task {
        Some(x) => Ok(HttpResponse::Ok().json(&x)),
        None => Err(common::SquadOvError::NotFound)
    }
}