use squadov_common::{
    SquadOvError,
    stripe::{
        webhook::{
            StripeGenericWebhookEvent,
            StripeSignature,
            StripeTypedWebhookEvent,
        },
        checkout::{
            StripeCheckoutSession,
        },
        invoice::{
            StripeInvoice,
            StripeInvoiceLineItem,
        },
        subscription::{
            StripeSubscription,
        }
    },
    subscriptions::{
        self,
        SquadOvSubTiers,
    },
    user,
};
use actix_web::{
    web,
    HttpResponse,
};
use crate::api::ApiApplication;
use std::{
    sync::Arc,
    convert::TryFrom,
    str::FromStr,
};
use uuid::Uuid;
use sqlx::{Executor, Postgres};

async fn update_user_subscription_from_line_item<'a, T>(ex: T, app: Arc<ApiApplication>, user_id: i64, d: &StripeInvoiceLineItem) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    if let Some(sub) = d.subscription.as_ref() {
        // Go from price to product to figure out what subscription tier we should give the user.
        let price = app.stripe.retrieve_a_price(&d.price.id).await?;
        let product = app.stripe.retrieve_a_product(&price.product).await?;
        let subscription = app.stripe.retrieve_a_subscription(&sub).await?;

        Ok(
            if let Some(tier) = product.metadata.get("tier") {
                let tier = SquadOvSubTiers::from_str(&tier)?;
                subscriptions::set_user_sub_tier(ex, user_id, tier, Some(subscription.current_period_end + chrono::Duration::days(2))).await?;

                if subscription.status.is_trial() {
                    sqlx::query!(
                        "
                        UPDATE squadov.users
                        SET last_trial_usage = NOW()
                        WHERE id = $1
                        ",
                        user_id,
                    )
                        .execute(&*app.pool)
                        .await?;
                }
                true
            } else {
                false
            }
        )
    } else {
        Err(SquadOvError::BadRequest)
    }
}

async fn handle_invoice_paid(app: Arc<ApiApplication>, event: StripeTypedWebhookEvent<StripeInvoice>) -> Result<(), SquadOvError> {
    // Invoice has been paid - find the customer and make sure we track them as having the proper subscription.
    if let Some(user_id) = subscriptions::get_user_id_from_stripe_customer_id(&*app.pool, &event.data.object.customer).await? {
        let user = user::get_squadov_user_from_id(&*app.pool, user_id).await?;
        for d in &event.data.object.lines.data {
            if update_user_subscription_from_line_item(&*app.pool, app.clone(), user_id, &d).await? {
                app.segment.track(&user.uuid.to_string(), "start_subscription").await?;
                app.discord.request_sync_user(user_id).await?;
                break;
            }
        }
        app.update_user_subscription(user_id).await?;
    }
    Ok(())
}

async fn handle_invoice_payment_failed(app: Arc<ApiApplication>, event: StripeTypedWebhookEvent<StripeInvoice>) -> Result<(), SquadOvError> {
    // Invoice has failed - find the customer and make sure we remove the subscription from them.
    let user_id = subscriptions::get_user_id_from_stripe_customer_id(&*app.pool, &event.data.object.customer).await?;
    if let Some(user_id) = user_id {
        let user = user::get_squadov_user_from_id(&*app.pool, user_id).await?;
        app.segment.track(&user.uuid.to_string(), "payment_failed").await?;
        let mut tx = app.pool.begin().await?;
        subscriptions::set_user_sub_tier(&mut tx, user_id, SquadOvSubTiers::Basic, None).await?;
        tx.commit().await?;
        app.update_user_subscription(user_id).await?;
    }
    Ok(())
}

async fn handle_checkout_session_completed(app: Arc<ApiApplication>, event: StripeTypedWebhookEvent<StripeCheckoutSession>) -> Result<(), SquadOvError> {
    // Not that we might be creating a new Stripe customer here and we should save the customer ID <> user ID relationship here.
    let mut tx = app.pool.begin().await?;

    let user = if let Some(client_reference_id) = event.data.object.client_reference_id.as_ref() {
        let user_uuid = Uuid::parse_str(client_reference_id)?;
        user::get_squadov_user_from_uuid(&*app.pool, &user_uuid).await?
    } else {
        return Err(SquadOvError::BadRequest);
    };

    if let Some(customer) = event.data.object.customer.as_ref() {
        subscriptions::associate_user_id_with_customer_id(&mut tx, user.id, &customer).await?;
    } else {
        return Err(SquadOvError::BadRequest);
    }

    // Also, payment is successful and we should create the subscription (probably for the first time here).
    if let Some(line_items) = event.data.object.line_items {
        for d in line_items.data {
            if update_user_subscription_from_line_item(&mut tx, app.clone(), user.id, &d).await? {
                app.segment.track(&user.uuid.to_string(), "start_subscription").await?;
                app.discord.request_sync_user(user.id).await?;
                break;
            }
        }
    }
    tx.commit().await?;
    
    app.update_user_subscription(user.id).await?;
    Ok(())
}

async fn handle_customer_subscription_update(app: Arc<ApiApplication>, event: StripeTypedWebhookEvent<StripeSubscription>) -> Result<(), SquadOvError> {
    let user_id = subscriptions::get_user_id_from_stripe_customer_id(&*app.pool, &event.data.object.customer).await?;

    if let Some(user_id) = user_id {
        let user = user::get_squadov_user_from_id(&*app.pool, user_id).await?;
        let mut tx = app.pool.begin().await?;
        if event.data.object.status.is_valid() {
            for d in &event.data.object.items.data {
                if update_user_subscription_from_line_item(&mut tx, app.clone(), user.id, &d).await? {
                    break;
                }
            }
        } else {
            app.segment.track(&user.uuid.to_string(), "end_subscription").await?;
            subscriptions::set_user_sub_tier(&mut tx, user_id, SquadOvSubTiers::Basic, None).await?;
        }
        tx.commit().await?;

        app.discord.request_sync_user(user_id).await?;
        app.update_user_subscription(user_id).await?;
    }

    Ok(())
}

pub async fn stripe_webhook_handler(app: web::Data<Arc<ApiApplication>>, payload: web::Bytes, sig: StripeSignature) -> Result<HttpResponse, SquadOvError> {
    // Pull the Stripe-Signature header and verify the signature to ensure that the request came from Stripe.
    let str_payload = String::from_utf8(payload.to_vec())?;
    if !sig.is_valid(&str_payload, &app.config.stripe.webhook_secret)? {
        return Err(SquadOvError::Unauthorized);
    }

    let event: StripeGenericWebhookEvent = serde_json::from_str(&str_payload)?; 
    log::info!("Handle Stripe Webhook: {} - {}", &event.id, &event.event_type);
    match event.event_type.as_str() {
        "invoice.paid" => handle_invoice_paid(app.as_ref().clone(), StripeTypedWebhookEvent::<StripeInvoice>::try_from(event)?).await?,
        "invoice.payment_failed" => handle_invoice_payment_failed(app.as_ref().clone(), StripeTypedWebhookEvent::<StripeInvoice>::try_from(event)?).await?,
        "checkout.session.completed" => handle_checkout_session_completed(app.as_ref().clone(), StripeTypedWebhookEvent::<StripeCheckoutSession>::try_from(event)?).await?,
        "customer.subscription.deleted" | "customer.subscription.updated" => handle_customer_subscription_update(app.as_ref().clone(), StripeTypedWebhookEvent::<StripeSubscription>::try_from(event)?).await?,
        _ => (),
    }

    Ok(HttpResponse::NoContent().finish())
}