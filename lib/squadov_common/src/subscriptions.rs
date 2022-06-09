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