use actix_web::{
    web,
    HttpResponse,
};
use crate::{
    api::{
        self,
        auth::SquadOVSession,
        v1::{
            self,
            FeatureFlags,
        },
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
            StripeCheckoutSubscriptionData,
        },
        currency::{
            StripeCurrency,
        },
        customer_portal::{
            StripeCreatePortalSessionRequest,
        },
        StripeApiClient,
    },
    subscriptions::{
        self,
        SquadOvFullPricingInfo,
        SquadOvSubTiers,
        SquadOvDiscount,
    },
    user::{
        self,
        SupportLevel,
    },
    rabbitmq::{
        RABBITMQ_LOW_PRIORITY,
        RABBITMQ_DEFAULT_PRIORITY,
        RABBITMQ_HIGH_PRIORITY,
    },
    vod::db as vdb,
};
use std::collections::HashMap;
use cached::{TimedCache, proc_macro::cached};
use serde::Deserialize;
use chrono::Utc;

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
    type = "TimedCache<(Option<i64>, bool, StripeCurrency), SquadOvFullPricingInfo>",
    create = "{ TimedCache::with_lifespan_and_capacity(600, 30) }",
    convert = r#"{ (user_id.clone(), annual, currency.clone()) }"#
)]
async fn get_subscription_pricing(app : Arc<api::ApiApplication>, user_id: Option<i64>, annual: bool, currency: StripeCurrency) -> Result<SquadOvFullPricingInfo, SquadOvError> {
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
                }),
                currency: Some(currency.clone()),
            }).await?;

            if let Some(pr) = pricing.data.first() {
                info.pricing.insert(
                    tier,
                    pr.unit_amount as f64 / 100.0 / if annual { 12.0 } else { 1.0 },
                );
            }
        }
    }

    Ok(info)
}

#[derive(Deserialize)]
pub struct SubscriptionQuery {
    pub annual: bool,
    #[serde(default="StripeCurrency::default")]
    pub currency: StripeCurrency,
}

pub async fn get_subscription_pricing_handler(app : web::Data<Arc<api::ApiApplication>>, session: Option<SquadOVSession>, query: web::Query<SubscriptionQuery>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        get_subscription_pricing(app.get_ref().clone(), session.map(|x| { x.user.id }), query.annual, query.currency.clone()).await?
    ))
}

#[derive(Deserialize)]
pub struct CheckoutQuery {
    pub tier: SquadOvSubTiers,
    pub annual: bool,
    #[serde(default="StripeCurrency::default")]
    pub currency: StripeCurrency,
}

pub async fn start_subscription_checkout_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession, query: web::Query<CheckoutQuery>) -> Result<HttpResponse, SquadOvError> {
    // If the user has an active subscription, then ship them to the customer portal instead.
    let current_tier = subscriptions::get_user_sub_tier(&*app.pool, session.user.id).await?;
    if current_tier.has_subscription() {
        start_subscription_manage_handler(app, session).await
    } else {
        // Check for any user discounts here.
        let discounts = get_largest_discount_for_user(app.get_ref().clone(), app.stripe.clone(), session.user.id).await?;
        let can_do_trial = if let Some(last_trial) = session.user.last_trial_usage.as_ref() {
            Utc::now().signed_duration_since(last_trial.clone()).num_days() >= 365
        } else {
            true
        };

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
                }),
                currency: Some(query.currency.clone()),
            }).await?;

            if let Some(price) = pricing.data.pop() {
                let existing_customer = subscriptions::get_user_stripe_customer_id(&*app.pool, session.user.id).await?;
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
                            subscription: None,
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
                    client_reference_id: Some(session.user.uuid.to_string()),
                    customer_email: if existing_customer.is_none() { Some(session.user.email.clone()) } else { None },
                    customer: existing_customer,
                    subscription_data: if can_do_trial {
                        Some(StripeCheckoutSubscriptionData{
                            trial_period_days: Some(7),
                        })
                    } else {
                        None
                    },
                    allow_promotion_codes: discounts.is_empty(),
                }).await?;

                Ok(HttpResponse::Ok().json(session.url))
            } else {
                Err(SquadOvError::NotFound)
            }
        } else {
            Err(SquadOvError::NotFound)
        }
    }
}

pub async fn start_subscription_manage_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession) -> Result<HttpResponse, SquadOvError> {
    let session = app.stripe.create_a_portal_session(StripeCreatePortalSessionRequest{
        customer: subscriptions::get_user_stripe_customer_id(&*app.pool, session.user.id).await?.ok_or(SquadOvError::NotFound)?,
        return_url: Some(format!("{}/settings", &app.config.squadov.app_url)),
    }).await?;

    Ok(HttpResponse::Ok().json(session.url))
}

pub async fn get_user_tier_handler(app : web::Data<Arc<api::ApiApplication>>, session: SquadOVSession) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        subscriptions::get_user_sub_tier(&*app.pool, session.user.id).await?
    ))
}

impl api::ApiApplication {
    pub async fn update_user_subscription(&self, user_id: i64) -> Result<(), SquadOvError> {
        let tier = subscriptions::get_user_sub_tier(&*self.pool, user_id).await?;
        let flags = v1::get_feature_flags(&*self.pool, user_id).await?;

        let mut tx = self.pool.begin().await?;
        match tier {
            SquadOvSubTiers::Basic => {
                user::update_user_support_priority(&mut tx, user_id, SupportLevel::Normal).await?;
                v1::update_feature_flags(&mut tx, user_id, FeatureFlags{
                    max_record_pixel_y: 720,
                    max_record_fps: 60,
                    max_bitrate_kbps: 6000,
                    mandatory_watermark: true,
                    vod_priority: RABBITMQ_LOW_PRIORITY as i16,
                    early_access: false,
                    vod_retention: Some(chrono::Duration::days(7).num_seconds()),
                    max_squad_size: Some(20),
                    max_clip_seconds: 120,
                    ..flags
                }).await?;
            },
            SquadOvSubTiers::Silver => {
                user::update_user_support_priority(&mut tx, user_id, SupportLevel::Normal).await?;
                v1::update_feature_flags(&mut tx, user_id, FeatureFlags{
                    max_record_pixel_y: 1080,
                    max_record_fps: 60,
                    max_bitrate_kbps: 12000,
                    mandatory_watermark: false,
                    vod_priority: RABBITMQ_DEFAULT_PRIORITY as i16,
                    early_access: false,
                    vod_retention: None,
                    max_squad_size: Some(100),
                    max_clip_seconds: 180,
                    ..flags
                }).await?;
            },
            SquadOvSubTiers::Gold => {
                user::update_user_support_priority(&mut tx, user_id, SupportLevel::High).await?;
                v1::update_feature_flags(&mut tx, user_id, FeatureFlags{
                    max_record_pixel_y: 1440,
                    max_record_fps: 60,
                    max_bitrate_kbps: 18000,
                    mandatory_watermark: false,
                    vod_priority: RABBITMQ_DEFAULT_PRIORITY as i16,
                    early_access: true,
                    vod_retention: None,
                    max_squad_size: None,
                    max_clip_seconds: 300,
                    ..flags
                }).await?;
            },
            SquadOvSubTiers::Diamond => {
                user::update_user_support_priority(&mut tx, user_id, SupportLevel::High).await?;
                v1::update_feature_flags(&mut tx, user_id, FeatureFlags{
                    max_record_pixel_y: 99999,
                    max_record_fps: 144,
                    max_bitrate_kbps: 24000,
                    mandatory_watermark: false,
                    vod_priority: RABBITMQ_HIGH_PRIORITY as i16,
                    early_access: true,
                    vod_retention: None,
                    max_squad_size: None,
                    max_clip_seconds: 300,
                    ..flags
                }).await?;
            },
        }
        vdb::update_user_vods_expiration_from_feature_flags(&mut tx, user_id).await?;
        v1::edit_user_max_squad_size_from_feature_flags(&mut tx, user_id).await?;
        tx.commit().await?;
        Ok(())
    }
}