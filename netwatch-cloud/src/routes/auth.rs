use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub account_id: Uuid,
    pub api_key: String,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    if req.email.is_empty() || !req.email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }
    if req.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password_hash = bcrypt::hash(&req.password, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let account_id = Uuid::new_v4();
    let trial_ends_at = chrono::Utc::now() + chrono::Duration::days(14);
    let retention_days = 3; // trial plan: 72 hours (3 days)

    sqlx::query(
        "INSERT INTO accounts (id, email, password_hash, trial_ends_at, retention_days) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(account_id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(trial_ends_at)
    .bind(retention_days)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("register failed: {}", e);
        StatusCode::CONFLICT // email already exists
    })?;

    // Create Stripe Customer if configured
    if let Some(ref stripe_key) = state.config.stripe_secret_key {
        match ureq::post("https://api.stripe.com/v1/customers")
            .set("Authorization", &format!("Bearer {}", stripe_key))
            .send_form(&[
                ("email", req.email.as_str()),
                ("metadata[account_id]", &account_id.to_string()),
            ]) {
            Ok(resp) => {
                if let Ok(body) = resp.into_json::<serde_json::Value>() {
                    if let Some(cust_id) = body["id"].as_str() {
                        let _ = sqlx::query(
                            "UPDATE accounts SET stripe_customer_id = $1 WHERE id = $2",
                        )
                        .bind(cust_id)
                        .bind(account_id)
                        .execute(&state.db)
                        .await;
                        tracing::info!("created stripe customer {} for account {}", cust_id, account_id);
                    }
                }
            }
            Err(e) => {
                tracing::error!("failed to create stripe customer: {}", e);
            }
        }
    }

    // Create first API key
    let raw_key = format!("nw_ak_{}", generate_random_string(32));
    let key_prefix = &raw_key[..14];
    let key_hash = bcrypt::hash(&raw_key, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO api_keys (account_id, key_hash, key_prefix, label) VALUES ($1, $2, $3, $4)"
    )
    .bind(account_id)
    .bind(&key_hash)
    .bind(key_prefix)
    .bind("default")
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create default alert rules
    let default_rules: &[(&str, &str, &str, Option<f64>, Option<&str>, i32, &str)] = &[
        ("Host offline", "host_status", "changes_to", None, Some("offline"), 60, "critical"),
        ("High packet loss", "gateway_loss_pct", ">", Some(5.0), None, 60, "warning"),
        ("High gateway latency", "gateway_rtt_ms", ">", Some(100.0), None, 60, "warning"),
        ("High DNS latency", "dns_rtt_ms", ">", Some(200.0), None, 60, "info"),
    ];

    for (name, metric, condition, threshold, threshold_str, duration_secs, severity) in default_rules {
        let _ = sqlx::query(
            "INSERT INTO alert_rules (account_id, name, metric, condition, threshold, threshold_str, duration_secs, severity) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        )
        .bind(account_id)
        .bind(name)
        .bind(metric)
        .bind(condition)
        .bind(threshold)
        .bind(threshold_str)
        .bind(duration_secs)
        .bind(severity)
        .execute(&state.db)
        .await;
    }

    // Create tokens
    let access_token = crate::auth::create_access_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let refresh_token = crate::auth::create_refresh_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterResponse {
        account_id,
        api_key: raw_key,
        access_token,
        refresh_token,
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub account_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, password_hash FROM accounts WHERE email = $1"
    )
    .bind(&req.email)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    let (account_id, hash) = row;

    if !bcrypt::verify(&req.password, &hash).unwrap_or(false) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let access_token = crate::auth::create_access_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let refresh_token = crate::auth::create_refresh_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { 
        account_id, 
        access_token, 
        refresh_token,
    }))
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, StatusCode> {
    // Verify the refresh token
    let claims = crate::auth::verify_refresh_token(&req.refresh_token, &state.config.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let account_id = claims.sub;

    // Verify the account still exists
    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1)")
        .bind(account_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !exists {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Generate new tokens with rotation
    let new_access_token = crate::auth::create_access_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let new_refresh_token = crate::auth::create_refresh_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RefreshResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
    }))
}

fn generate_random_string(len: usize) -> String {
    let mut result = String::with_capacity(len);
    while result.len() < len {
        let chunk = uuid::Uuid::new_v4().simple().to_string();
        result.push_str(&chunk);
    }
    result.truncate(len);
    result
}
