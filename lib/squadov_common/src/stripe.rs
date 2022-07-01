pub mod product;
pub mod price;
pub mod checkout;
pub mod coupon;
pub mod customer_portal;
pub mod invoice;
pub mod webhook;
pub mod subscription;
pub mod currency;
pub mod customer;

use crate::{
    SquadOvError,
};
use reqwest::{
    Client,
    ClientBuilder,
    header,
    Request,
    Response,
};
use rand::{Rng, SeedableRng};
use serde::{Deserializer, Deserialize};

#[derive(Clone, Debug)]
pub enum StripeApiVersion {
    V20200827,
    Unknown,
}

impl ToString for StripeApiVersion {
    fn to_string(&self) -> String {
        match self {
            StripeApiVersion::V20200827 => "2020-08-27",
            StripeApiVersion::Unknown => "Unknown",
        }.to_string()
    }
}

impl<'de> Deserialize<'de> for StripeApiVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "2020-08-27" => StripeApiVersion::V20200827,
            _ => StripeApiVersion::Unknown,
        })
    }
}

pub struct StripeApiClient {
    pub config: StripeApiConfig,
    client: Client,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StripeApiConfig {
//    publishable_api_key: String,
    secret_api_key: String,
    api_version: StripeApiVersion,
    pub webhook_secret: String,
}

impl StripeApiClient {
    pub fn new(config: &StripeApiConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        headers.insert(header::AUTHORIZATION, header::HeaderValue::from_str(&format!("Bearer {}", &config.secret_api_key)).unwrap());
        headers.insert("Stripe-Version", header::HeaderValue::from_str(&config.api_version.to_string()).unwrap());

        StripeApiClient {
            config: config.clone(),
            client: ClientBuilder::new()
                .default_headers(headers)
                .timeout(std::time::Duration::from_secs(120))
                .connect_timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap()
        }
    }

    pub fn build_url(path: &str) -> String {
        format!("https://api.stripe.com/{}", path)
    }

    pub async fn send_request(&self, request: Request) -> Result<Response, SquadOvError> {
        let mut rng = rand::rngs::StdRng::from_entropy();
        for i in 0u32..5u32 {
            if let Some(request) = request.try_clone() {
                let resp = self.client.execute(request).await?;
                let status = resp.status().as_u16();
                if status == 429 {
                    async_std::task::sleep(std::time::Duration::from_millis(100u64 * 2u64.pow(i) + rng.gen_range(0..1000))).await;
                    continue;
                } else if status < 300 {
                    return Ok(resp);
                } else {
                    return Err(SquadOvError::InternalError(format!("Failed Stripe Request - {} - {}", status, resp.text().await?)));
                }
            } else {
                log::error!("Uncloneable request...can't auto retry.");
                return Err(SquadOvError::BadRequest);
            }
        }

        Err(SquadOvError::RateLimit)
    }
}