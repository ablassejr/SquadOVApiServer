use crate::{
    SquadOvError,
    stripe::StripeApiClient,
};
use std::collections::HashMap;
use std::iter::FromIterator;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct StripeCreatePortalSessionRequest {
    pub customer: String,
    pub return_url: Option<String>,
}

impl StripeCreatePortalSessionRequest {
    fn to_map(&self) -> HashMap<String, String> {
        // Would be nice to generalize this eventually.
        let mut tuples: Vec<(String, String)> = vec![
            ("customer".to_string(), self.customer.clone())
        ];

        if let Some(return_url) = self.return_url.as_ref() {
            tuples.push(
                ("return_url".to_string(), return_url.clone()),
            );
        }

        HashMap::from_iter(tuples)
    }
}

#[derive(Deserialize)]
pub struct StripePortalSession {
    pub url: String,
}

impl StripeApiClient {
    pub async fn create_a_portal_session(&self, request: StripeCreatePortalSessionRequest) -> Result<StripePortalSession, SquadOvError> {
        Ok(
            self.send_request(
                self.client.post(&Self::build_url("v1/billing_portal/sessions"))
                    .form(&request.to_map())
                    .build()?
            )
                .await?
                .json::<StripePortalSession>().await?
        )
    }
}