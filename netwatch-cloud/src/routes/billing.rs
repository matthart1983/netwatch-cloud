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
    pub has_slack_webhook: bool,
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

    // Issue #11: Don't expose slack webhook URL - return only a boolean
    let has_slack_webhook = slack_webhook.is_some();

    let portal_url = if let (Some(ref cust_id), Some(ref key)) =
        (&stripe_customer_id, &state.config.stripe_secret_key)
    {
        create_portal_session(cust_id, key).await.ok()
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
        has_slack_webhook,
        portal_url,
    }))
}

#[derive(Deserialize)]
pub struct UpdateAccount {
    pub notify_email: Option<bool>,
    pub slack_webhook: Option<String>,
}

fn normalize_slack_webhook(webhook: &str) -> Result<Option<&str>, StatusCode> {
    let webhook = webhook.trim();

    if webhook.is_empty() {
        return Ok(None);
    }

    if !webhook.starts_with("https://hooks.slack.com/") {
        tracing::warn!("invalid slack webhook URL attempted");
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(Some(webhook))
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
        let value = normalize_slack_webhook(webhook)?;
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
        create_portal_session(cust_id, key).await.ok()
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

fn create_portal_session_blocking(customer_id: &str, secret_key: &str) -> Result<String, String> {
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

// Issue #18: Use async wrapper to avoid blocking Tokio runtime with ureq (synchronous HTTP)
async fn create_portal_session(customer_id: &str, secret_key: &str) -> Result<String, String> {
    let cust_id = customer_id.to_string();
    let key = secret_key.to_string();
    
    tokio::task::spawn_blocking(move || {
        create_portal_session_blocking(&cust_id, &key)
    })
    .await
    .map_err(|e| format!("task join error: {}", e))?
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

    // Issue #10: Webhook secret is REQUIRED - fail if not configured
    let secret = match &state.config.stripe_webhook_secret {
        Some(s) => s,
        None => {
            tracing::error!("stripe webhook secret not configured");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    // Always verify signature - never skip verification
    let sig_header = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_signature(payload, sig_header, secret) {
        tracing::error!("stripe webhook signature verification failed");
        return StatusCode::BAD_REQUEST;
    }

    let event: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let event_type = event["type"].as_str().unwrap_or("");
    let event_id = event["id"].as_str().unwrap_or("");
    let data_object = &event["data"]["object"];

    tracing::info!("stripe webhook: {} (event_id: {})", event_type, event_id);

    // Check for idempotency: if event already processed, return 200 immediately
    let already_processed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM webhook_events WHERE event_id = $1)"
    )
    .bind(event_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    if already_processed {
        tracing::info!("stripe webhook: event {} already processed, returning 200", event_id);
        return StatusCode::OK;
    }

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

    // Handle result: return 500 on error, 200 on success
    match result {
        Ok(()) => {
            // Record event as processed
            let _ = sqlx::query(
                "INSERT INTO webhook_events (event_id, event_type) VALUES ($1, $2)"
            )
            .bind(event_id)
            .bind(event_type)
            .execute(&state.db)
            .await;
            StatusCode::OK
        }
        Err(e) => {
            tracing::error!("stripe webhook handler error: {}", e);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use axum::extract::State;
    use axum::Json;
    use sqlx::postgres::PgPoolOptions;
    use std::time::UNIX_EPOCH;
    use uuid::Uuid;

    fn test_state(stripe_webhook_secret: Option<String>) -> Arc<AppState> {
        Arc::new(AppState {
            db: PgPoolOptions::new()
                .connect_lazy("postgres://postgres:postgres@localhost/netwatch_test")
                .expect("lazy test pool"),
            config: ServerConfig {
                database_url: "postgres://postgres:postgres@localhost/netwatch_test".to_string(),
                jwt_secret: "test-jwt-secret".to_string(),
                bind_addr: "127.0.0.1:0".to_string(),
                app_url: "https://app.example.com".to_string(),
                resend_api_key: None,
                stripe_secret_key: Some("sk_test_123".to_string()),
                stripe_webhook_secret,
            },
        })
    }

    #[test]
    fn test_account_info_serialization_hides_slack_webhook() {
        let account = AccountInfo {
            email: "user@example.com".to_string(),
            created_at: Utc::now(),
            plan: "trial".to_string(),
            trial_ends_at: None,
            stripe_customer_id: Some("cus_123".to_string()),
            notify_email: true,
            has_slack_webhook: true,
            portal_url: Some("https://billing.stripe.com/session/test".to_string()),
        };

        let payload = serde_json::to_value(account).expect("serialize account info");
        assert_eq!(payload["has_slack_webhook"], serde_json::Value::Bool(true));
        assert!(payload.get("slack_webhook").is_none());
    }

    #[tokio::test]
    async fn test_stripe_webhook_requires_configured_secret() {
        let state = test_state(None);

        let status = stripe_webhook(
            State(state),
            HeaderMap::new(),
            Bytes::from_static(br#"{"id":"evt_123","type":"invoice.payment_failed","data":{"object":{"customer":"cus_123"}}}"#),
        )
        .await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_verify_signature_accepts_valid_signature() {
        let secret = "whsec_test";
        let payload = r#"{"id":"evt_123","type":"invoice.payment_failed","data":{"object":{"customer":"cus_123"}}}"#;
        let timestamp = UNIX_EPOCH.elapsed().expect("unix time").as_secs();
        let signed_content = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac");
        mac.update(signed_content.as_bytes());
        let sig_header = format!("t={},v1={}", timestamp, hex::encode(mac.finalize().into_bytes()));

        assert!(verify_signature(payload, &sig_header, secret));
    }

    #[test]
    fn test_verify_signature_rejects_old_timestamp() {
        let secret = "whsec_test";
        let payload = r#"{"id":"evt_123","type":"invoice.payment_failed","data":{"object":{"customer":"cus_123"}}}"#;
        let timestamp = UNIX_EPOCH
            .elapsed()
            .expect("unix time")
            .as_secs()
            .saturating_sub(301);
        let signed_content = format!("{}.{}", timestamp, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("hmac");
        mac.update(signed_content.as_bytes());
        let sig_header = format!("t={},v1={}", timestamp, hex::encode(mac.finalize().into_bytes()));

        assert!(!verify_signature(payload, &sig_header, secret));
    }

    #[tokio::test]
    async fn test_update_account_rejects_invalid_slack_webhook() {
        let result = update_account(
            AuthUser {
                account_id: Uuid::new_v4(),
            },
            State(test_state(Some("whsec_test".to_string()))),
            Json(UpdateAccount {
                notify_email: None,
                slack_webhook: Some("http://localhost:6379".to_string()),
            }),
        )
        .await;

        assert_eq!(result, Err(StatusCode::BAD_REQUEST));
    }
}
