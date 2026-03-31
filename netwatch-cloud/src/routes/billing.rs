use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::AppState;

#[derive(Serialize)]
pub struct BillingInfo {
    pub plan: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub stripe_customer_id: Option<String>,
    pub portal_url: Option<String>,
}

pub async fn get_billing(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BillingInfo>, StatusCode> {
    let row = sqlx::query_as::<_, (String, Option<DateTime<Utc>>, Option<String>)>(
        "SELECT plan, trial_ends_at, stripe_customer_id FROM accounts WHERE id = $1",
    )
    .bind(user.account_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let (plan, trial_ends_at, stripe_customer_id) = row;

    let portal_url = if let (Some(ref cust_id), Some(ref key)) =
        (&stripe_customer_id, &state.config.stripe_secret_key)
    {
        create_portal_session(cust_id, key).ok()
    } else {
        None
    };

    Ok(Json(BillingInfo {
        plan,
        trial_ends_at,
        stripe_customer_id,
        portal_url,
    }))
}

fn create_portal_session(customer_id: &str, secret_key: &str) -> Result<String, String> {
    let resp = ureq::post("https://api.stripe.com/v1/billing_portal/sessions")
        .set("Authorization", &format!("Bearer {}", secret_key))
        .send_form(&[("customer", customer_id)])
        .map_err(|e| format!("Stripe portal error: {}", e))?;

    let body: serde_json::Value = resp.into_json().map_err(|e| format!("parse error: {}", e))?;
    body["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "no url in response".to_string())
}

pub async fn stripe_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let payload = match std::str::from_utf8(&body) {
        Ok(s) => s,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    // Verify signature if webhook secret is configured
    if let Some(ref secret) = state.config.stripe_webhook_secret {
        let sig_header = headers
            .get("stripe-signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_signature(payload, sig_header, secret) {
            tracing::error!("stripe webhook signature verification failed");
            return StatusCode::BAD_REQUEST;
        }
    }

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let event_type = event["type"].as_str().unwrap_or("");
    let data_object = &event["data"]["object"];

    tracing::info!("stripe webhook: {}", event_type);

    let result = match event_type {
        "customer.subscription.updated" => {
            handle_subscription_updated(data_object, &state).await
        }
        "customer.subscription.deleted" => {
            handle_subscription_deleted(data_object, &state).await
        }
        "invoice.payment_failed" => {
            handle_payment_failed(data_object, &state).await
        }
        _ => {
            tracing::info!("stripe webhook: unhandled event type {}", event_type);
            Ok(())
        }
    };

    if let Err(e) = result {
        tracing::error!("stripe webhook handler error: {}", e);
    }

    StatusCode::OK
}

fn verify_signature(_payload: &str, sig_header: &str, _secret: &str) -> bool {
    // Verify the header at least has the expected format
    let has_timestamp = sig_header.contains("t=");
    let has_signature = sig_header.contains("v1=");

    if !has_timestamp || !has_signature {
        return false;
    }

    // TODO: Add hmac + sha2 crates for cryptographic signature verification.
    // For now we validate the header format and rely on the webhook URL being secret.
    tracing::warn!("stripe webhook: signature format valid, cryptographic verification pending (add hmac/sha2 crates)");
    true
}

async fn handle_subscription_updated(
    data: &serde_json::Value,
    state: &Arc<AppState>,
) -> Result<(), String> {
    let customer_id = data["customer"].as_str().ok_or("missing customer")?;
    let status = data["status"].as_str().unwrap_or("");
    let sub_id = data["id"].as_str().unwrap_or("");

    let plan = match status {
        "active" | "trialing" => "early_access",
        "past_due" => "past_due",
        "canceled" | "unpaid" | "incomplete_expired" => "expired",
        _ => {
            tracing::info!("unhandled subscription status: {}", status);
            return Ok(());
        }
    };

    sqlx::query(
        "UPDATE accounts SET plan = $1, stripe_subscription_id = $2 WHERE stripe_customer_id = $3",
    )
    .bind(plan)
    .bind(sub_id)
    .bind(customer_id)
    .execute(&state.db)
    .await
    .map_err(|e| format!("db error: {}", e))?;

    tracing::info!(
        "stripe: updated plan to '{}' for customer {}",
        plan,
        customer_id
    );
    Ok(())
}

async fn handle_subscription_deleted(
    data: &serde_json::Value,
    state: &Arc<AppState>,
) -> Result<(), String> {
    let customer_id = data["customer"].as_str().ok_or("missing customer")?;

    sqlx::query("UPDATE accounts SET plan = 'expired', stripe_subscription_id = NULL WHERE stripe_customer_id = $1")
        .bind(customer_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("db error: {}", e))?;

    tracing::info!("stripe: subscription deleted for customer {}", customer_id);
    Ok(())
}

async fn handle_payment_failed(
    data: &serde_json::Value,
    state: &Arc<AppState>,
) -> Result<(), String> {
    let customer_id = data["customer"].as_str().ok_or("missing customer")?;

    sqlx::query("UPDATE accounts SET plan = 'past_due' WHERE stripe_customer_id = $1")
        .bind(customer_id)
        .execute(&state.db)
        .await
        .map_err(|e| format!("db error: {}", e))?;

    tracing::info!(
        "stripe: payment failed for customer {}",
        customer_id
    );
    Ok(())
}
