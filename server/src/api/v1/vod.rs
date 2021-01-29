mod create;
mod delete;
mod find;
mod get;

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Serialize,Deserialize};

#[derive(Serialize,Deserialize)]
pub struct VodAssociation {
    #[serde(rename = "matchUuid")]
    pub match_uuid: Option<Uuid>,
    #[serde(rename = "userUuid")]
    pub user_uuid: Option<Uuid>,
    #[serde(rename = "videoUuid")]
    pub video_uuid: Uuid,
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>
}

pub use create::*;
pub use delete::*;
pub use find::*;
pub use get::*;