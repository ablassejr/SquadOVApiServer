use crate::{
    SquadOvError,
    stripe::{
        StripeApiClient,
        invoice::StripeInvoiceLineContainer,
    },
};
use serde::{
    Serialize,
    Serializer,
    Deserialize,
    Deserializer,
    de::Error,
};
use derive_more::{Display};
use std::collections::HashMap;
use std::iter::FromIterator;

#[derive(Display)]
pub enum StripeCheckoutSessionMode {
    #[display(fmt="payment")]
    Payment,
    #[display(fmt="setup")]
    Setup,
    #[display(fmt="subscription")]
    Subscription,
}

impl Serialize for StripeCheckoutSessionMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StripeCheckoutSessionMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "payment" => StripeCheckoutSessionMode::Payment,
            "setup" => StripeCheckoutSessionMode::Setup,
            "subscription" => StripeCheckoutSessionMode::Subscription,
            _ => return Err(D::Error::custom("Invalid")),
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct StripeCheckoutLineItem {
    pub price: String,
    pub quantity: Option<i32>,
    pub subscription: Option<String>,
}

#[derive(Serialize)]
pub struct StripeCheckoutDiscount {
    pub coupon: Option<String>,
    pub promotion_code: Option<String>,
}

#[derive(Serialize)]
pub struct StripeCreateSessionRequest {
    pub cancel_url: String,
    pub mode: StripeCheckoutSessionMode,
    pub success_url: String,
    pub client_reference_id: Option<String>,
    pub customer: Option<String>,
    pub customer_email: Option<String>,
    pub line_items: Vec<StripeCheckoutLineItem>,
    pub discounts: Vec<StripeCheckoutDiscount>,
}

impl StripeCreateSessionRequest {
    fn to_map(&self) -> HashMap<String, String> {
        // Would be nice to generalize this eventually.
        let mut tuples: Vec<(String, String)> = vec![
            ("success_url".to_string(), self.success_url.clone()),
            ("cancel_url".to_string(), self.cancel_url.clone()),
            ("mode".to_string(), format!("{}", &self.mode)),
        ];

        if let Some(c) = self.client_reference_id.as_ref() {
            tuples.push(("client_reference_id".to_string(), c.clone()));
        }

        if let Some(c) = self.customer.as_ref() {
            tuples.push(("customer".to_string(), c.clone()));
        }

        if let Some(c) = self.customer_email.as_ref() {
            tuples.push(("customer_email".to_string(), c.clone()));
        }

        for (i, li) in self.line_items.iter().enumerate() {
            tuples.push(
                (format!("line_items[{}][price]", i), li.price.clone()),
            );

            if let Some(quantity) = li.quantity {
                tuples.push(
                    (format!("line_items[{}][quantity]", i), format!("{}", quantity)),
                );
            }
        }

        for (i, di) in self.discounts.iter().enumerate() {
            if let Some(c) = di.coupon.as_ref() {
                tuples.push(
                    (format!("discounts[{}][coupon]", i), c.clone()),
                );
            }

            if let Some(p) = di.promotion_code.as_ref() {
                tuples.push(
                    (format!("discounts[{}][promotion_code]", i), p.clone()),
                );
            }
        }

        HashMap::from_iter(tuples)
    }
}

#[derive(Deserialize)]
pub struct StripeCheckoutSession {
    pub client_reference_id: Option<String>,
    pub customer: Option<String>,
    pub url: Option<String>,
    pub line_items: Option<StripeInvoiceLineContainer>,
}

impl StripeApiClient {
    pub async fn create_a_session(&self, request: StripeCreateSessionRequest) -> Result<StripeCheckoutSession, SquadOvError> {
        Ok(
            self.send_request(
                self.client.post(&Self::build_url("v1/checkout/sessions"))
                    .form(&request.to_map())
                    .build()?
            )
                .await?
                .json::<StripeCheckoutSession>().await?
        )
    }
}