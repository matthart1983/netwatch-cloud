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

    sqlx::query(
        "INSERT INTO accounts (id, email, password_hash) VALUES ($1, $2, $3)"
    )
    .bind(account_id)
    .bind(&req.email)
    .bind(&password_hash)
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("register failed: {}", e);
        StatusCode::CONFLICT // email already exists
    })?;

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

    Ok(Json(RegisterResponse {
        account_id,
        api_key: raw_key,
    }))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub account_id: Uuid,
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

    let token = crate::auth::create_token(account_id, &state.config.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse { token, account_id }))
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
