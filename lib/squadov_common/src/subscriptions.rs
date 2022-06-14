use serde::{Serializer, Serialize, Deserialize, Deserializer, de::Error};
use chrono::{DateTime, Utc};
use sqlx::{Executor, Postgres};
use std::collections::HashMap;
use crate::SquadOvError;
use derive_more::{Display};
use serde_with::{serde_as, DisplayFromStr};
use std::str::FromStr;

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

#[derive(Eq, PartialEq, Display, Hash, Clone)]
pub enum SquadOvSubTiers {
    #[display(fmt="BASIC")]
    Basic,
    #[display(fmt="SILVER")]
    Silver,
    #[display(fmt="GOLD")]
    Gold,
    #[display(fmt="DIAMOND")]
    Diamond,   
}

impl FromStr for SquadOvSubTiers {
    type Err = SquadOvError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "BASIC" => SquadOvSubTiers::Basic,
            "SILVER" => SquadOvSubTiers::Silver,
            "GOLD" => SquadOvSubTiers::Gold,
            "DIAMOND" => SquadOvSubTiers::Diamond,
            _ => return Err(SquadOvError::BadRequest),
        })
    }
}

impl SquadOvSubTiers {
    pub fn has_subscription(&self) -> bool {
        match self {
            SquadOvSubTiers::Basic => false,
            _ => true
        }
    }
}

pub async fn get_user_sub_tier<'a, T>(ex: T, user_id: i64) -> Result<SquadOvSubTiers, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT tier
            FROM squadov.user_subscription_tier
            WHERE user_id = $1
                AND end_tm >= NOW()
            ",
            user_id
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                SquadOvSubTiers::from_str(&x.tier).unwrap_or(SquadOvSubTiers::Basic)
            })
            .unwrap_or(SquadOvSubTiers::Basic)
    )
}

pub async fn set_user_sub_tier<'a, T>(ex: T, user_id: i64, tier: SquadOvSubTiers, end_tm: Option<DateTime<Utc>>) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.user_subscription_tier (
            user_id,
            tier,
            start_tm,
            end_tm
        ) VALUES (
            $1,
            $2,
            NOW(),
            $3
        ) ON CONFLICT (user_id) DO UPDATE
            SET tier = EXCLUDED.tier,
                start_tm = EXCLUDED.start_tm,
                end_tm = EXCLUDED.end_tm
        ",
        user_id,
        &format!("{}", tier),
        end_tm,
    )
        .execute(ex)
        .await?;
    Ok(())
}

impl Serialize for SquadOvSubTiers {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for SquadOvSubTiers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(
            match SquadOvSubTiers::from_str(s.to_uppercase().as_str()) {
                Ok(x) => x,
                Err(_) => return Err(D::Error::custom("Invalid")),
            }
        )
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct SquadOvDiscount {
    #[serde(skip_serializing)]
    pub id: String,
    pub percent: f64,
    pub reason: String,
}

#[serde_as]
#[derive(Serialize, Clone)]
pub struct SquadOvFullPricingInfo {
    #[serde_as(as="HashMap<DisplayFromStr, _>")]
    pub pricing: HashMap<SquadOvSubTiers, f64>,
    pub discounts: Vec<SquadOvDiscount>,
}

pub async fn get_user_stripe_customer_id<'a, T>(ex: T, user_id: i64) -> Result<Option<String>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT customer
            FROM squadov.stripe_customers
            WHERE user_id = $1
            ",
            user_id
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.customer
            })
    )
}

pub async fn get_user_id_from_stripe_customer_id<'a, T>(ex: T, customer_id: &str) -> Result<Option<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT user_id
            FROM squadov.stripe_customers
            WHERE customer = $1
            ",
            customer_id,
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.user_id
            })
    )
}

pub async fn associate_user_id_with_customer_id<'a, T>(ex: T, user_id: i64, customer_id: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.stripe_customers (user_id, customer)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        ",
        user_id,
        customer_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}