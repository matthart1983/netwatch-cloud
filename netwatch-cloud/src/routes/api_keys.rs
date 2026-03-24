use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::AppState;

#[derive(Serialize)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub key_prefix: String,
    pub label: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub async fn list_keys(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApiKeyInfo>>, StatusCode> {
    let keys = sqlx::query_as::<_, (Uuid, String, Option<String>, DateTime<Utc>, Option<DateTime<Utc>>)>(
        "SELECT id, key_prefix, label, created_at, last_used_at FROM api_keys WHERE account_id = $1 ORDER BY created_at"
    )
    .bind(user.account_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<ApiKeyInfo> = keys
        .into_iter()
        .map(|(id, key_prefix, label, created_at, last_used_at)| {
            ApiKeyInfo { id, key_prefix, label, created_at, last_used_at }
        })
        .collect();

    Ok(Json(result))
}

#[derive(Serialize)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub api_key: String,
}

pub async fn create_key(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CreateKeyResponse>, StatusCode> {
    let raw_key = format!("nw_ak_{}", {
        let mut s = String::new();
        while s.len() < 32 {
            s.push_str(&uuid::Uuid::new_v4().simple().to_string());
        }
        s.truncate(32);
        s
    });
    let key_prefix = &raw_key[..14];
    let key_hash = bcrypt::hash(&raw_key, 10)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO api_keys (id, account_id, key_hash, key_prefix) VALUES ($1, $2, $3, $4)"
    )
    .bind(id)
    .bind(user.account_id)
    .bind(&key_hash)
    .bind(key_prefix)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateKeyResponse { id, api_key: raw_key }))
}

pub async fn delete_key(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        "DELETE FROM api_keys WHERE id = $1 AND account_id = $2"
    )
    .bind(id)
    .bind(user.account_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(StatusCode::NO_CONTENT)
}
