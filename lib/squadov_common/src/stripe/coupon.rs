use crate::{
    SquadOvError,
    stripe::StripeApiClient,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct StripeCoupon {
    pub name: String,
    pub percent_off: Option<f64>,
}

impl StripeApiClient {
    pub async fn retrieve_a_coupon(&self, request: &str) -> Result<StripeCoupon, SquadOvError> {
        Ok(
            self.send_request(
                self.client.get(&Self::build_url(&format!("v1/coupons/{}", request)))
                    .build()?
            )
                .await?
                .json::<StripeCoupon>().await?
        )
    }
}