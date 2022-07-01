use crate::{
    SquadOvError,
    stripe::StripeApiClient,
};
use serde::{Deserialize};
use std::collections::HashMap;
use std::iter::FromIterator;

#[derive(Deserialize)]
pub struct StripeCustomer {
    pub id: String,
    pub email: String,
}

impl StripeApiClient {
    pub async fn retrieve_a_customer(&self, id: &str) -> Result<StripeCustomer, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url(&format!("v1/customers/{}", id)))
                    .build()?
            )
                .await?
                .json::<StripeCustomer>().await?
        )
    }

    pub async fn create_a_customer(&self, email: &str) -> Result<StripeCustomer, SquadOvError> {
        Ok(
            self.send_request(
                self.client.post(&Self::build_url("v1/customers"))
                    .form(&HashMap::<String, String>::from_iter(vec![
                        ("email".to_string(), email.to_string())
                    ]))
                    .build()?
            )
                .await?
                .json::<StripeCustomer>().await?
        )
    }
}