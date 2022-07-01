use serde::{
    Serialize,
    Serializer,
    Deserialize,
    Deserializer,
    de::Error,
};
use derive_more::{Display};
use crate::{
    SquadOvError,
    stripe::{
        StripeApiClient,
        invoice::StripeInvoiceLineContainer,
    },
};
use chrono::{DateTime, Utc, serde::ts_seconds};

#[derive(Display)]
pub enum StripeSubscriptionStatus {
    #[display(fmt="incomplete")]
    Incomplete,
    #[display(fmt="incomplete_expired")]
    IncompleteExpired,
    #[display(fmt="trialing")]
    Trialing,
    #[display(fmt="active")]
    Active,
    #[display(fmt="past_due")]
    PastDue,
    #[display(fmt="canceled")]
    Canceled,
    #[display(fmt="unpaid")]
    Unpaid,
}

impl Serialize for StripeSubscriptionStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StripeSubscriptionStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "incomplete" => StripeSubscriptionStatus::Incomplete,
            "incomplete_expired" => StripeSubscriptionStatus::IncompleteExpired,
            "trialing" => StripeSubscriptionStatus::Trialing,
            "active" => StripeSubscriptionStatus::Active,
            "past_due" => StripeSubscriptionStatus::PastDue,
            "canceled" => StripeSubscriptionStatus::Canceled,
            "unpaid" => StripeSubscriptionStatus::Unpaid,
            _ => return Err(D::Error::custom("Invalid")),
        })
    }
}

impl StripeSubscriptionStatus {
    pub fn is_valid(&self) -> bool {
        match self {
            StripeSubscriptionStatus::Active | StripeSubscriptionStatus::Trialing => true,
            _ => false
        }
    }

    pub fn is_trial(&self) -> bool {
        match self {
            StripeSubscriptionStatus::Trialing => true,
            _ => false
        }
    }
}

#[derive(Deserialize)]
pub struct StripeSubscription {
    pub customer: String,
    pub items: StripeInvoiceLineContainer,
    pub status: StripeSubscriptionStatus,

    #[serde(with="ts_seconds")]
    pub current_period_end: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct StripeListSubscriptionsRequest {
    pub customer: Option<String>,
}

#[derive(Deserialize)]
pub struct StripeListSubscriptionsResponse {
    pub data: Vec<StripeSubscription>,
}

impl StripeApiClient {
    pub async fn retrieve_a_subscription(&self, subscription: &str) -> Result<StripeSubscription, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url(&format!("v1/subscriptions/{}", subscription)))
                    .build()?
            )
                .await?
                .json::<StripeSubscription>().await?
        )
    }

    pub async fn list_subscriptions(&self, request: StripeListSubscriptionsRequest) -> Result<Vec<StripeSubscription>, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url("v1/subscriptions"))
                    .query(&if let Some(customer) = request.customer {
                        vec![("customer", customer)]
                    } else {
                        vec![]
                    })
                    .build()?
            )
                .await?
                .json::<StripeListSubscriptionsResponse>()
                .await?
                .data
        )
    }
}