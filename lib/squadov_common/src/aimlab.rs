use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::SquadOvError;
use sqlx::{Executor, Postgres};

#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct AimlabTask {
    pub id: i64,
    #[serde(rename = "userId", default)]
    pub user_id: i64,
    #[serde(rename = "klutchId")]
    pub klutch_id: String,
    #[serde(rename = "matchUuid", default)]
    pub match_uuid: Uuid,
    #[serde(rename = "taskName")]
    pub task_name: String,
    pub mode: i32,
    pub score: i64,
    pub version: String,
    #[serde(rename = "createDate")]
    pub create_date: DateTime<Utc>,
    #[serde(rename = "rawData")]
    pub raw_data: serde_json::Value
}

pub async fn is_user_aimlab_task_owner<'a, T>(ex: T, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM squadov.aimlab_tasks
                WHERE match_uuid = $1
                    AND user_id = $2
            ) AS "exists!"
            "#,
            match_uuid,
            user_id,
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

pub async fn list_aimlab_matches_for_uuids<'a, T>(ex: T, uuids: &[Uuid]) -> Result<Vec<AimlabTask>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            AimlabTask,
            "
            SELECT *
            FROM squadov.aimlab_tasks
            WHERE match_uuid = ANY($1)
            ",
            uuids
        )
            .fetch_all(ex)
            .await?
    )
}