use serde::{de::DeserializeOwned, Deserialize};
use chrono::{DateTime, Utc, NaiveDateTime};
use std::collections::HashMap;
use actix_web::{
    HttpRequest,
    FromRequest,
    dev,
};
use futures_util::future::{ok, err, Ready};
use crate::{
    SquadOvError,
};
use hmac::{Hmac, Mac, NewMac};
use sha2::Sha256;
use std::convert::TryFrom;

#[derive(Deserialize)]
pub struct StripeGenericWebhookData {
    pub object: serde_json::Value,
}

#[derive(Deserialize)]
pub struct StripeGenericWebhookEvent {
    pub id: String,
    #[serde(rename="type")]
    pub event_type: String,
    pub data: StripeGenericWebhookData,
}

#[derive(Deserialize)]
pub struct StripeTypedWebhookData<T> {
    pub object: T,
}

#[derive(Deserialize)]
pub struct StripeTypedWebhookEvent<T> {
    pub id: String,
    #[serde(rename="type")]
    pub event_type: String,
    pub data: StripeTypedWebhookData<T>,
}

impl<T> TryFrom<StripeGenericWebhookEvent> for StripeTypedWebhookEvent<T>
where
    T: DeserializeOwned
{
    type Error = SquadOvError;

    fn try_from(e: StripeGenericWebhookEvent) -> Result<Self, Self::Error> {
        Ok(
            Self {
                id: e.id,
                event_type: e.event_type,
                data: StripeTypedWebhookData{
                    object: serde_json::from_value(e.data.object)?
                },
            }
        )
    }
}

#[derive(Debug)]
pub struct StripeSignature {
    pub t: DateTime<Utc>,
    pub v: HashMap<String, String>,
}

impl StripeSignature {
    pub fn is_valid(&self, payload: &str, endpoint_secret: &str) -> Result<bool, SquadOvError> {
        let signed_payload = format!("{}.{}", self.t.timestamp(), payload);

        let mut mac = Hmac::<Sha256>::new_from_slice(endpoint_secret.as_bytes())?;
        mac.update(signed_payload.as_bytes());

        let test_sig = hex::encode(mac.finalize().into_bytes());
        Ok(
            if let Some(ref_sig) = self.v.get("v1") {
                ref_sig.as_str() == test_sig.as_str()
            } else {
                false
            }
        )
    }
}

impl FromRequest for StripeSignature {
    type Error = SquadOvError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req : &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        if let Some(sig) = req.headers().get("Stripe-Signature") {
            let mut parsed_sig = StripeSignature{
                t: Utc::now(),
                v: HashMap::new(),
            };

            for c in sig.to_str().unwrap_or("").split(",") {
                let prop: Vec<_> = c.split("=").collect();
                if prop.len() < 2 {
                    continue;
                }

                match prop[0] {
                    "t" => {
                        let ts = prop[1].parse::<i64>().unwrap_or(0);
                        parsed_sig.t = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(ts, 0), Utc);
                    },
                    "v1" => {
                        parsed_sig.v.insert(
                            prop[0].to_string(),
                            prop[1].to_string(),
                        );
                    },
                    _ => (),
                }
            }

            ok(parsed_sig)
        } else {
            err(SquadOvError::BadRequest)
        }
    }
}
