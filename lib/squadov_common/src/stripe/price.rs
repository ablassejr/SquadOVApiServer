use crate::{
    SquadOvError,
    stripe::{
        StripeApiClient,
        currency::StripeCurrency,
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

#[derive(Display, Clone)]
pub enum StripeRecurringInterval {
    #[display(fmt="day")]
    Day,
    #[display(fmt="week")]
    Week,
    #[display(fmt="month")]
    Month,
    #[display(fmt="year")]
    Year,
}

impl Serialize for StripeRecurringInterval {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StripeRecurringInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "day" => StripeRecurringInterval::Day,
            "week" => StripeRecurringInterval::Week,
            "month" => StripeRecurringInterval::Month,
            "year" => StripeRecurringInterval::Year,
            _ => return Err(D::Error::custom("Invalid")),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StripeRecurring {
    pub interval: Option<StripeRecurringInterval>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StripePrice {
    pub id: String,
    pub unit_amount: i64,
    pub recurring: Option<StripeRecurring>,
    pub product: String,
}

pub struct ListAllPricesRequest {
    pub product: Option<String>,
    pub recurring: Option<StripeRecurring>,
    pub currency: Option<StripeCurrency>,
}

#[derive(Deserialize)]
pub struct ListAllPricesResponse {
    pub data: Vec<StripePrice>,
}

impl StripeApiClient {
    pub async fn list_all_prices(&self, request: ListAllPricesRequest) -> Result<ListAllPricesResponse, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url("v1/prices"))
                    .query(&if let Some(product) = request.product {
                        vec![("product", product)]
                    } else {
                        vec![]
                    })
                    .query(&if let Some(recurring) = request.recurring {
                        if let Some(interval) = recurring.interval {
                            vec![("recurring[interval]", interval)]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    })
                    .query(&if let Some(c) = request.currency {
                        vec![("currency", format!("{}", c))]
                    } else {
                        vec![]
                    })
                    .build()?
            )
                .await?
                .json::<ListAllPricesResponse>().await?
        )
    }

    pub async fn retrieve_a_price(&self, price: &str) -> Result<StripePrice, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url(&format!("v1/prices/{}", price)))
                    .build()?
            )
                .await?
                .json::<StripePrice>().await?
        )
    }
}