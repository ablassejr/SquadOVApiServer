use crate::{
    SquadOvError,
    stripe::StripeApiClient,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct StripeProduct {
    pub id: String,
    pub metadata: HashMap<String, String>,
}

pub struct StripeListAllProductRequest {
    pub active: Option<bool>,
}

#[derive(Deserialize)]
pub struct StripeListAllProductResponse {
    pub data: Vec<StripeProduct>,
}

impl StripeApiClient {
    pub async fn list_all_products(&self, request: StripeListAllProductRequest) -> Result<StripeListAllProductResponse, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url("v1/products"))
                    .query(&[("active", request.active)])
                    .build()?
            )
                .await?
                .json::<StripeListAllProductResponse>().await?
        )
    }
}