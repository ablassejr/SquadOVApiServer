pub mod demo;
pub mod parser;
pub mod data_table;
pub mod entity;
pub mod prop;
pub mod prop_types;
pub mod math;
pub mod weapon;
pub mod gsi;
pub mod db;
pub mod schema;
pub mod rabbitmq;
pub mod summary;

use crate::SquadOvError;
use sqlx::{Executor, Postgres};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoListQuery {
    modes: Option<Vec<String>>,
    maps: Option<Vec<String>>,
    has_vod: Option<bool>,
    has_demo: Option<bool>,
}

pub async fn is_user_in_csgo_match<'a, T>(ex: T, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1
                FROM squadov.csgo_match_views
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