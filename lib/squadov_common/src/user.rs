use serde::Serialize;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::{
    SquadOvError,
};
use sqlx::{Executor, Postgres};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all="camelCase")]
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
    pub registration_time: Option<DateTime<Utc>>,
    pub support_priority: String,
    pub last_trial_usage: Option<DateTime<Utc>>,
}

pub enum SupportLevel {
    Normal,
    High
}

impl ToString for SupportLevel {
    fn to_string(&self) -> String {
        match self {
            SupportLevel::Normal => "normal",
            SupportLevel::High => "high"
        }.to_string()
    }
}

pub async fn update_user_support_priority<'a, T>(ex: T, id: i64, level: SupportLevel) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.users
        SET support_priority = $2
        WHERE id = $1
        ",
        id,
        &level.to_string(),
    )
        .execute(ex)
        .await?;
    Ok(())
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
                registration_time,
                support_priority,
                last_trial_usage
            FROM squadov.users
            WHERE uuid = $1
            ",
            uuid,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_squadov_user_from_id<'a, T>(ex: T, id: i64) -> Result<SquadOVUser, SquadOvError>
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
                registration_time,
                support_priority,
                last_trial_usage
            FROM squadov.users
            WHERE id = $1
            ",
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_squadov_user_from_email<'a, T>(ex: T, email: &str) -> Result<SquadOVUser, SquadOvError>
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
                registration_time,
                support_priority,
                last_trial_usage
            FROM squadov.users
            WHERE email = $1
            ",
            email,
        )
            .fetch_one(ex)
            .await?
    )
}