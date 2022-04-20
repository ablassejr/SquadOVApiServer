use serde::Serialize;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::{
    SquadOvError,
};
use sqlx::{Executor, Postgres};

#[derive(Debug, Serialize, Clone)]
pub struct SquadOVUser {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub verified: bool,
    pub uuid: Uuid,
    #[serde(skip_serializing)]
    pub is_test: bool,
    #[serde(skip_serializing)]
    pub is_admin: bool,
    #[serde(skip_serializing)]
    pub welcome_sent: bool,
    #[serde(rename="registrationTime")]
    pub registration_time: Option<DateTime<Utc>>,
}

pub async fn get_squadov_user_from_uuid<'a, T>(ex: T, uuid: &Uuid) -> Result<SquadOVUser, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            SquadOVUser,
            "
            SELECT
                id,
                username,
                email,
                verified,
                uuid,
                is_test,
                is_admin,
                welcome_sent,
                registration_time
            FROM squadov.users
            WHERE uuid = $1
            ",
            uuid,
        )
            .fetch_one(ex)
            .await?
    )
}