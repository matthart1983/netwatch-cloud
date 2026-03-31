use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::SystemTime;

use crate::auth::AuthUser;
use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

// --- Account GET/PUT ---

#[derive(Serialize)]
pub struct AccountInfo {
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub plan: String,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub stripe_customer_id: Option<String>,
    pub notify_email: bool,
    pub slack_webhook: Option<String>,
    pub portal_url: Option<String>,
}

pub async fn get_account(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<AccountInfo>, StatusCode> {
    let row = sqlx::query_as::<_, (String, DateTime<Utc>, String, Option<DateTime<Utc>>, Option<String>, bool, Option<String>)>(
        "SELECT email, created_at, plan, trial_ends_at, stripe_customer_id, notify_email, slack_webhook FROM accounts WHERE id = $1",
    )
    .bind(user.account_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let (email, created_at, plan, trial_ends_at, stripe_customer_id, notify_email, slack_webhook) = row;

    let portal_url = if let (Some(ref cust_id), Some(ref key)) =
        (&stripe_customer_id, &state.config.stripe_secret_key)
    {
        create_portal_session(cust_id, key).ok()
    } else {
        None
    };

    Ok(Json(AccountInfo {
        email,
        created_at,
        plan,
        trial_ends_at,
        stripe_customer_id,
        notify_email,
        slack_webhook,
        portal_url,
    }))
}

#[derive(Deserialize)]
pub struct UpdateAccount {
    pub notify_email: Option<bool>,
    pub slack_webhook: Option<String>,
}

pub async fn update_account(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateAccount>,
) -> Result<StatusCode, StatusCode> {
    if let Some(notify) = req.notify_email {
        sqlx::query("UPDATE accounts SET notify_email = $1 WHERE id = $2")
            .bind(notify)
            .bind(user.account_id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(ref webhook) = req.slack_webhook {
        let value = if webhook.is_empty() { None } else { Some(webhook.as_str()) };
        sqlx::query("UPDATE accounts SET slack_webhook = $1 WHERE id = $2")
            .bind(value)
            .bind(user.account_id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

// --- Billing ---

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

    let event_id = event["id"].as_str().unwrap_or("");
    let event_type = event["type"].as_str().unwrap_or("");
    let data_object = &event["data"]["object"];

    tracing::info!("stripe webhook: {} (event_id: {})", event_type, event_id);

    // FIX #4: Check if webhook already processed for idempotency
    // This prevents duplicate processing if Stripe retries the webhook
    if event_id.is_empty() {
        tracing::warn!("stripe webhook: missing event_id");
        return StatusCode::BAD_REQUEST;
    }

    let already_processed: bool = sqlx::query_scalar(
        "SELECT processed FROM webhook_events WHERE event_id = $1"
    )
    .bind(event_id)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None)
    .unwrap_or(false);

    if already_processed {
        tracing::info!("stripe webhook: event {} already processed", event_id);
        return StatusCode::OK;
    }

    // FIX #4: Insert event record before processing to mark as in-progress
    // Use INSERT OR IGNORE pattern to handle race conditions
    let _ = sqlx::query(
        "INSERT INTO webhook_events (event_id, event_type, processed) VALUES ($1, $2, false)
         ON CONFLICT (event_id) DO NOTHING"
    )
    .bind(event_id)
    .bind(event_type)
    .execute(&state.db)
    .await;

    // Process the webhook event
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

    // FIX #5: Return error status on processing failure to let Stripe retry
    match result {
        Ok(()) => {
            // Mark event as processed
            let _ = sqlx::query(
                "UPDATE webhook_events SET processed = true WHERE event_id = $1"
            )
            .bind(event_id)
            .execute(&state.db)
            .await;
            
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("stripe webhook handler error: {}", e);
            // FIX #5: Return 500 on handler error to trigger Stripe retry
            // Do NOT return 200 - Stripe should retry this webhook
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn verify_signature(payload: &str, sig_header: &str, secret: &str) -> bool {
    // Parse signature header: t=<timestamp>,v1=<signature>,v1=<signature>...
    let mut timestamp: Option<u64> = None;
    let mut signatures: Vec<String> = Vec::new();

    for part in sig_header.split(',') {
        let trimmed = part.trim();
        if let Some(ts) = trimmed.strip_prefix("t=") {
            if let Ok(t) = ts.parse::<u64>() {
                timestamp = Some(t);
            }
        } else if let Some(sig) = trimmed.strip_prefix("v1=") {
            signatures.push(sig.to_string());
        }
    }

    // Extract timestamp or reject
    let timestamp = match timestamp {
        Some(t) => t,
        None => {
            tracing::warn!("stripe webhook: missing timestamp in signature header");
            return false;
        }
    };

    // Check timestamp is within 5 minutes (prevent replay attacks)
    let now = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => {
            tracing::error!("stripe webhook: system time error");
            return false;
        }
    };

    let time_diff = if now > timestamp { now - timestamp } else { timestamp - now };
    if time_diff > 300 {
        // 300 seconds = 5 minutes
        tracing::warn!(
            "stripe webhook: timestamp too old or in future (diff: {} seconds)",
            time_diff
        );
        return false;
    }

    // Reject if no valid signatures found
    if signatures.is_empty() {
        tracing::warn!("stripe webhook: no v1 signatures found in header");
        return false;
    }

    // Reconstruct signed content: "<timestamp>.<payload>"
    let signed_content = format!("{}.{}", timestamp, payload);

    // Try to verify with any of the provided signatures
    for sig_hex in signatures {
        // Decode the hex-encoded signature
        let signature = match hex::decode(&sig_hex) {
            Ok(s) => s,
            Err(_) => {
                tracing::debug!("stripe webhook: failed to decode signature hex: {}", sig_hex);
                continue;
            }
        };

        // Compute HMAC-SHA256
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => {
                tracing::error!("stripe webhook: failed to create HMAC");
                return false;
            }
        };

        mac.update(signed_content.as_bytes());

        // Constant-time comparison to prevent timing attacks
        if mac.verify_slice(&signature).is_ok() {
            tracing::debug!("stripe webhook: signature verified successfully");
            return true;
        }
    }

    tracing::warn!("stripe webhook: no valid signatures found");
    false
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
