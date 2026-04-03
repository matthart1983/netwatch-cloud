use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub password: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let email = clean_email(&req.email);
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }
    if req.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password_hash = bcrypt::hash(&req.password, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let account_id = Uuid::new_v4();
    let trial_ends_at = Utc::now() + Duration::days(14);
    let retention_days = 3;

    sqlx::query(
        "INSERT INTO accounts (id, email, password_hash, trial_ends_at, retention_days) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(account_id)
    .bind(&email)
    .bind(&password_hash)
    .bind(trial_ends_at)
    .bind(retention_days)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("register failed: {}", e);
        StatusCode::CONFLICT
    })?;

    if let Some(ref stripe_key) = state.config.stripe_secret_key {
        match ureq::post("https://api.stripe.com/v1/customers")
            .set("Authorization", &format!("Bearer {}", stripe_key))
            .send_form(&[("email", email.as_str()), ("metadata[account_id]", &account_id.to_string())])
        {
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

    let raw_key = format!("nw_ak_{}", generate_random_string(32));
    let key_prefix = &raw_key[..14];
    let key_hash = bcrypt::hash(&raw_key, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO api_keys (account_id, key_hash, key_prefix, label) VALUES ($1, $2, $3, $4)",
    )
    .bind(account_id)
    .bind(&key_hash)
    .bind(key_prefix)
    .bind("default")
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let default_rules: &[(&str, &str, &str, Option<f64>, Option<&str>, i32, &str)] = &[
        ("Host offline", "host_status", "changes_to", None, Some("offline"), 60, "critical"),
        ("High packet loss", "gateway_loss_pct", ">", Some(5.0), None, 60, "warning"),
        ("High gateway latency", "gateway_rtt_ms", ">", Some(100.0), None, 60, "warning"),
        ("High DNS latency", "dns_rtt_ms", ">", Some(200.0), None, 60, "info"),
    ];

    for (name, metric, condition, threshold, threshold_str, duration_secs, severity) in default_rules {
        let _ = sqlx::query(
            "INSERT INTO alert_rules (account_id, name, metric, condition, threshold, threshold_str, duration_secs, severity) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
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

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let email = clean_email(&req.email);
    let row = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, password_hash FROM accounts WHERE email = $1",
    )
    .bind(&email)
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

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, StatusCode> {
    let claims = crate::auth::verify_refresh_token(&req.refresh_token, &state.config.jwt_secret)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let account_id = claims.sub;

    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1)")
        .bind(account_id)
        .fetch_one(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !exists {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let new_access_token = crate::auth::create_access_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let new_refresh_token = crate::auth::create_refresh_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RefreshResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
    }))
}

pub async fn forgot_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<StatusCode, StatusCode> {
    let email = clean_email(&req.email);
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let account = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, email FROM accounts WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some((account_id, account_email)) = account else {
        return Ok(StatusCode::NO_CONTENT);
    };

    let token = generate_random_string(64);
    let token_hash = hash_reset_token(&token);
    let expires_at = Utc::now() + Duration::hours(1);

    let _ = sqlx::query("DELETE FROM password_reset_tokens WHERE expires_at <= now() OR used_at IS NOT NULL")
        .execute(&state.db)
        .await;

    let _ = sqlx::query("DELETE FROM password_reset_tokens WHERE account_id = $1")
        .bind(account_id)
        .execute(&state.db)
        .await;

    sqlx::query(
        "INSERT INTO password_reset_tokens (account_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(account_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Err(err) = send_password_reset_email(&state, &account_email, &token) {
        tracing::error!("failed to send password reset email: {}", err);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn reset_password(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<StatusCode, StatusCode> {
    if req.token.trim().is_empty() || req.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let token_hash = hash_reset_token(req.token.trim());
    let row = sqlx::query_as::<_, (Uuid,)>(
        "SELECT account_id FROM password_reset_tokens WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::BAD_REQUEST)?;

    let account_id = row.0;
    let password_hash = bcrypt::hash(&req.password, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut tx = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE accounts SET password_hash = $1 WHERE id = $2")
        .bind(&password_hash)
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query("UPDATE password_reset_tokens SET used_at = now() WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

fn clean_email(email: &str) -> String {
    email.trim().to_string()
}

fn hash_reset_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

fn password_reset_url(base_url: &str, token: &str) -> String {
    format!("{}/reset-password?token={}", base_url.trim_end_matches('/'), token)
}

fn send_password_reset_email(state: &Arc<AppState>, email: &str, token: &str) -> Result<(), String> {
    let Some(api_key) = state.config.resend_api_key.as_ref() else {
        return Err("RESEND_API_KEY not configured".to_string());
    };

    let reset_url = password_reset_url(&state.config.app_url, token);
    let body = serde_json::json!({
        "from": "NetWatch <onboarding@resend.dev>",
        "to": [email],
        "subject": "Reset your NetWatch Cloud password",
        "text": format!(
            "We received a request to reset your NetWatch Cloud password.\n\nUse this link to choose a new password:\n{}\n\nThis link expires in 60 minutes. If you did not request this, you can ignore this email.",
            reset_url
        ),
        "html": format!(
            "<p>We received a request to reset your NetWatch Cloud password.</p><p><a href=\"{0}\">Reset your password</a></p><p>This link expires in 60 minutes. If you did not request this, you can ignore this email.</p><p>{0}</p>",
            reset_url
        ),
    });

    ureq::post("https://api.resend.com/emails")
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("resend request failed: {}", e))?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_reset_token_is_stable() {
        assert_eq!(hash_reset_token("abc"), hash_reset_token("abc"));
        assert_ne!(hash_reset_token("abc"), hash_reset_token("abcd"));
    }

    #[test]
    fn test_password_reset_url_trims_trailing_slash() {
        let url = password_reset_url("https://app.example.com/", "token123");
        assert_eq!(url, "https://app.example.com/reset-password?token=token123");
    }
}
