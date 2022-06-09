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
        product::{
            StripeListAllProductRequest,
            StripeSearchProductsRequest,
        },
        price::{
            ListAllPricesRequest,
            StripeRecurring,
            StripeRecurringInterval,
        },
        checkout::{
            StripeCreateSessionRequest,
            StripeCheckoutSessionMode,
            StripeCheckoutLineItem,
            StripeCheckoutDiscount,
        },
        StripeApiClient,
    },
    subscriptions::{
        self,
        SquadOvFullPricingInfo,
        SquadOvSubTiers,
        SquadOvDiscount,
    },
};
use std::collections::HashMap;
use cached::{TimedCache, proc_macro::cached};
use serde::Deserialize;

#[cached(
    result=true,
    type = "TimedCache<i64, Vec<SquadOvDiscount>>",
    create = "{ TimedCache::with_lifespan_and_capacity(600, 30) }",
    convert = r#"{ user_id }"#
)]
async fn get_largest_discount_for_user(app: Arc<api::ApiApplication>, stripe: Arc<StripeApiClient>, user_id: i64) -> Result<Vec<SquadOvDiscount>, SquadOvError> {
    let user_discounts: Vec<String> = sqlx::query!(
        "
        SELECT coupon
        FROM squadov.stripe_user_coupons
        where user_id = $1
        ",
        user_id,
    )
        .fetch_all(&*app.pool)
        .await?
        .into_iter()
        .map(|x| { x.coupon })
        .collect();

    // If multiple coupons are relevant here, get the one with the largest discount.
    // Assume we only use percentage discounts here.
    let mut highest_discount: Option<SquadOvDiscount> = None;
    for d in user_discounts {
        let coupon = stripe.retrieve_a_coupon(&d).await?;
        if let Some(pd) = coupon.percent_off {
            let nd = SquadOvDiscount{
                id: d,
                reason: coupon.name.clone(),
                percent: pd / 100.0,
            };

            if let Some(hd) = highest_discount.as_ref() {
                if nd.percent > hd.percent {
                    highest_discount = Some(nd);    
                }
            } else {
                highest_discount = Some(nd);
            }
        }
    }

    Ok(
        if let Some(hd) = highest_discount {
            vec![hd]
        } else {
            vec![]
        }
    )
}

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
        discounts: if let Some(user_id) = user_id { get_largest_discount_for_user(app.clone(), app.stripe.clone(), user_id).await? } else { vec![] },
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

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub tier: SquadOvSubTiers,
    pub annual: bool,
}

pub async fn start_subscription_checkout_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession, query: web::Query<CheckoutQuery>) -> Result<HttpResponse, SquadOvError> {
    // Check for any user discounts here.
    let discounts = get_largest_discount_for_user(app.get_ref().clone(), app.stripe.clone(), session.user.id).await?;

    let mut products = app.stripe.search_products(StripeSearchProductsRequest{
        active: Some(true),
        metadata: Some(HashMap::from([
            ("tier".to_string(), format!("{}", query.tier))
        ]))
    }).await?;

    if let Some(p) = products.data.pop() {
        let mut pricing = app.stripe.list_all_prices(ListAllPricesRequest{
            product: Some(p.id.clone()),
            recurring: Some(StripeRecurring{
                interval: Some(
                    if query.annual {
                        StripeRecurringInterval::Year
                    } else {
                        StripeRecurringInterval::Month
                    }
                )
            })
        }).await?;

        if let Some(price) = pricing.data.pop() {
            // Now we have the product + price + any potential discounts we want to apply.
            // We can go ahead and create the Stripe checkout session.
            let session = app.stripe.create_a_session(StripeCreateSessionRequest{
                cancel_url: format!("{}/subscription?success=0&tier={}&annual={}", &app.config.squadov.app_url, &query.tier, query.annual),
                success_url: format!("{}/subscription?success=1&tier={}&annual={}", &app.config.squadov.app_url, &query.tier, query.annual),
                mode: StripeCheckoutSessionMode::Subscription,
                line_items: vec![
                    StripeCheckoutLineItem{
                        price: price.id,
                        quantity: Some(1),
                    }
                ],
                discounts: if let Some(d) = discounts.first() {
                    vec![StripeCheckoutDiscount{
                        coupon: Some(d.id.clone()),
                        promotion_code: None,
                    }]
                } else {
                    vec![]
                },
            }).await?;

            Ok(HttpResponse::Ok().json(session.url))
        } else {
            Err(SquadOvError::NotFound)
        }
    } else {
        Err(SquadOvError::NotFound)
    }
}

pub async fn get_user_tier_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        subscriptions::get_user_sub_tier(&*app.pool, session.user.id).await?
    ))
}