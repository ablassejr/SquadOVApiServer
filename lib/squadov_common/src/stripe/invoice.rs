use serde::{
    Serialize,
    Serializer,
    Deserialize,
    Deserializer,
    de::Error,
};
use derive_more::{Display};
use crate::{
    stripe::{
        price::StripePrice,
    },
};

#[derive(Display)]
pub enum StripeInvoiceStatus {
    #[display(fmt="draft")]
    Draft,
    #[display(fmt="open")]
    Open,
    #[display(fmt="paid")]
    Paid,
    #[display(fmt="uncollectible")]
    Uncollectible,
    #[display(fmt="void")]
    Void,
}

impl Serialize for StripeInvoiceStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StripeInvoiceStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "draft" => StripeInvoiceStatus::Draft,
            "open" => StripeInvoiceStatus::Open,
            "paid" => StripeInvoiceStatus::Paid,
            "uncollectible" => StripeInvoiceStatus::Uncollectible,
            "void" => StripeInvoiceStatus::Void,
            _ => return Err(D::Error::custom("Invalid")),
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct StripeInvoiceLineItem {
    pub price: StripePrice,
    pub quantity: Option<i32>,
    pub subscription: Option<String>,
}

#[derive(Deserialize)]
pub struct StripeInvoiceLineContainer {
    pub data: Vec<StripeInvoiceLineItem>,
}

#[derive(Deserialize)]
pub struct StripeInvoice {
    pub customer: String,
    pub subscription: Option<String>,
    pub lines: StripeInvoiceLineContainer,
}