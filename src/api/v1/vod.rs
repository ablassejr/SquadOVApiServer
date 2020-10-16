mod create;
mod delete;

use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize};

#[derive(Deserialize)]
pub struct VodAssociation {
    #[serde(rename = "matchUuid")]
    match_uuid: Uuid,
    #[serde(rename = "userUuid")]
    user_uuid: Uuid,
    #[serde(rename = "vodUuid")]
    vod_uuid: Uuid,
    #[serde(rename = "startTime")]
    start_time: DateTime<Utc>,
    #[serde(rename = "endTime")]
    end_time: DateTime<Utc>
}

pub use create::*;
pub use delete::*;