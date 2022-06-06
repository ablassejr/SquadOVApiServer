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

#[derive(Default)]
pub struct StripeSearchProductsRequest {
    pub active: Option<bool>,
    pub metadata: Option<HashMap<String, String>>,
}

impl StripeSearchProductsRequest {
    fn build_query(&self) -> String {
        let mut parts: Vec<String> = vec![];
        
        if let Some(active) = self.active.as_ref() {
            parts.push(format!("active:'{}'", active));
        }

        if let Some(metadata) = self.metadata.as_ref() {
            for (k, v) in metadata {
                parts.push(format!("metadata['{}']:'{}'", k, v));
            }
        }

        parts.join(" AND ")
    }
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

    pub async fn search_products(&self, request: StripeSearchProductsRequest) -> Result<StripeListAllProductResponse, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url("v1/products/search"))
                    .query(&[("query", request.build_query())])
                    .build()?
            )
                .await?
                .json::<StripeListAllProductResponse>().await?
        )
    }
}