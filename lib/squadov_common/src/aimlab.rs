use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Serialize,Deserialize,Clone)]
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
