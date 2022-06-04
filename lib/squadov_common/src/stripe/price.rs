use crate::{
    SquadOvError,
    stripe::StripeApiClient,
};
use serde::{
    Serialize,
    Serializer,
    Deserialize,
    Deserializer,
    de::Error,
};
use derive_more::{Display};

#[derive(Display)]
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

#[derive(Deserialize)]
pub struct StripeRecurring {
    pub interval: Option<StripeRecurringInterval>,
}

#[derive(Deserialize)]
pub struct StripePrice {
    pub unit_amount: i64,
    pub recurring: Option<StripeRecurring>,
}

pub struct ListAllPricesRequest {
    pub product: Option<String>,
    pub recurring: Option<StripeRecurring>,
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
                    .build()?
            )
                .await?
                .json::<ListAllPricesResponse>().await?
        )
    }
}