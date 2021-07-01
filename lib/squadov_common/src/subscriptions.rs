use serde::Serialize;
use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use crate::SquadOvError;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct User2UserSubscription {
    pub id: i64,
    pub source_user_id: i64,
    pub dest_user_id: i64,
    pub is_twitch: bool,
    pub last_checked: DateTime<Utc>,
}

pub async fn get_u2u_subscription<'a, T>(ex: T, id: i64) -> Result<User2UserSubscription, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            User2UserSubscription,
            "
            SELECT *
            FROM squadov.user_to_user_subscriptions
            WHERE id = $1
            ",
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_u2u_subscription_from_user_ids<'a, T>(ex: T, src_user_id: i64, dest_user_id: i64) -> Result<Vec<User2UserSubscription>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            User2UserSubscription,
            "
            SELECT *
            FROM squadov.user_to_user_subscriptions
            WHERE source_user_id = $1
                AND dest_user_id = $2
            ",
            src_user_id,
            dest_user_id,
        )
            .fetch_all(ex)
            .await?
    )
}