use actix_web::{
    web,
    HttpResponse,
};
use crate::{
    api::{
        self,
        auth::SquadOVSession,
    },
};
use std::sync::Arc;
use std::str::FromStr;
use squadov_common::{
    SquadOvError,
    stripe::{
        product::StripeListAllProductRequest,
        price::{
            ListAllPricesRequest,
            StripeRecurring,
            StripeRecurringInterval,
        },
    },
    subscriptions::{
        SquadOvFullPricingInfo,
        SquadOvSubTiers,
    },
};
use std::collections::HashMap;
use cached::{TimedCache, proc_macro::cached};
use serde::Deserialize;

#[cached(
    result=true,
    type = "TimedCache<(Option<i64>, bool), SquadOvFullPricingInfo>",
    create = "{ TimedCache::with_lifespan_and_capacity(600, 30) }",
    convert = r#"{ (user_id.clone(), annual) }"#
)]
async fn get_subscription_pricing(app : Arc<api::ApiApplication>, user_id: Option<i64>, annual: bool) -> Result<SquadOvFullPricingInfo, SquadOvError> {
    let products = app.stripe.list_all_products(StripeListAllProductRequest{
        active: Some(true),
    }).await?;

    let mut info = SquadOvFullPricingInfo{
        pricing: HashMap::new(),
        discounts: vec![],
    };

    info.pricing.insert(SquadOvSubTiers::Basic, 0.0);
    for p in products.data {
        if let Some(tier) = p.metadata.get("tier") {
            let tier = SquadOvSubTiers::from_str(&tier)?;
            info.pricing.insert(
                tier,
                {
                    let pricing = app.stripe.list_all_prices(ListAllPricesRequest{
                        product: Some(p.id.clone()),
                        recurring: Some(StripeRecurring{
                            interval: Some(
                                if annual {
                                    StripeRecurringInterval::Year
                                } else {
                                    StripeRecurringInterval::Month
                                }
                            )
                        })
                    }).await?;

                    pricing.data.first().ok_or(SquadOvError::BadRequest)?.unit_amount as f64 / 100.0 / if annual { 12.0 } else { 1.0 }
                }
            );
        }
    }

    if let Some(user_id) = user_id {

    }

    Ok(info)
}

#[derive(Deserialize)]
pub struct SubscriptionQuery {
    pub annual: bool
}

pub async fn get_subscription_pricing_handler(app : web::Data<Arc<api::ApiApplication>>, session: Option<SquadOVSession>, query: web::Query<SubscriptionQuery>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        get_subscription_pricing(app.get_ref().clone(), session.map(|x| { x.user.id }), query.annual).await?
    ))
}