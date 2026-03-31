use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum TokenType {
    #[serde(rename = "access")]
    Access,
    #[serde(rename = "refresh")]
    Refresh,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub token_type: TokenType,
    pub exp: usize,
}

pub fn create_access_token(account_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(15))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: account_id,
        token_type: TokenType::Access,
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn create_refresh_token(account_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: account_id,
        token_type: TokenType::Refresh,
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Legacy function - now uses access token expiry
#[deprecated(since = "0.1.0", note = "use create_access_token instead")]
#[allow(dead_code)]
pub fn create_token(account_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    create_access_token(account_id, secret)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn verify_refresh_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let claims = verify_token(token, secret)?;
    
    // Ensure it's actually a refresh token
    if !matches!(claims.token_type, TokenType::Refresh) {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    
    Ok(claims)
}

/// Extractor for authenticated web users (JWT)
pub struct AuthUser {
    pub account_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = verify_token(token, &state.config.jwt_secret)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser {
            account_id: claims.sub,
        })
    }
}

/// Extractor for agent API key auth
pub struct AgentAuth {
    pub account_id: Uuid,
    pub api_key_id: Uuid,
}

impl FromRequestParts<Arc<AppState>> for AgentAuth {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let api_key = auth_header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Extract prefix for lookup
        if api_key.len() < 12 || !api_key.starts_with("nw_ak_") {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let prefix = &api_key[..14]; // "nw_ak_" + 8 chars

        // Look up by prefix, then bcrypt verify
        let row = sqlx::query_as::<_, (Uuid, Uuid, String)>(
            "SELECT id, account_id, key_hash FROM api_keys WHERE key_prefix = $1"
        )
        .bind(prefix)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

        let (key_id, account_id, hash) = row;

        if !bcrypt::verify(api_key, &hash).unwrap_or(false) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // Update last_used_at
        let _ = sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = $1")
            .bind(key_id)
            .execute(&state.db)
            .await;

        Ok(AgentAuth {
            account_id,
            api_key_id: key_id,
        })
    }
}
