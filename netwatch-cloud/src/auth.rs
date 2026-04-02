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

pub fn create_access_token(account_id: Uuid, secret: &str) -> Result<String, String> {
    // Issue #20: Avoid unwrap on time arithmetic - use proper error handling
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(15))
        .ok_or_else(|| "token expiry calculation overflowed".to_string())?
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
    .map_err(|e| e.to_string())
}

pub fn create_refresh_token(account_id: Uuid, secret: &str) -> Result<String, String> {
    // Issue #20: Avoid unwrap on time arithmetic - use proper error handling
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .ok_or_else(|| "token expiry calculation overflowed".to_string())?
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
    .map_err(|e| e.to_string())
}

/// Legacy function - now uses access token expiry
#[deprecated(since = "0.1.0", note = "use create_access_token instead")]
#[allow(dead_code)]
pub fn create_token(account_id: Uuid, secret: &str) -> Result<String, String> {
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

pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let claims = verify_token(token, secret)?;
    
    // Ensure it's actually an access token
    if !matches!(claims.token_type, TokenType::Access) {
        return Err(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ));
    }
    
    Ok(claims)
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

fn api_key_prefix(api_key: &str) -> Option<&str> {
    if !api_key.is_ascii() || !api_key.starts_with("nw_ak_") {
        return None;
    }

    api_key.get(..14)
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

        let claims = verify_access_token(token, &state.config.jwt_secret)
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

        let prefix = api_key_prefix(api_key).ok_or(StatusCode::UNAUTHORIZED)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_access_token_accepts_access_token() {
        let secret = "test-secret";
        let token = create_access_token(Uuid::new_v4(), secret).unwrap();
        let claims = verify_access_token(&token, secret).unwrap();
        assert!(matches!(claims.token_type, TokenType::Access));
    }

    #[test]
    fn test_verify_access_token_rejects_refresh_token() {
        let secret = "test-secret";
        let token = create_refresh_token(Uuid::new_v4(), secret).unwrap();
        let result = verify_access_token(&token, secret);
        assert!(result.is_err(), "verify_access_token should reject refresh tokens");
    }

    #[test]
    fn test_verify_refresh_token_accepts_refresh_token() {
        let secret = "test-secret";
        let token = create_refresh_token(Uuid::new_v4(), secret).unwrap();
        let claims = verify_refresh_token(&token, secret).unwrap();
        assert!(matches!(claims.token_type, TokenType::Refresh));
    }

    #[test]
    fn test_verify_refresh_token_rejects_access_token() {
        let secret = "test-secret";
        let token = create_access_token(Uuid::new_v4(), secret).unwrap();
        let result = verify_refresh_token(&token, secret);
        assert!(result.is_err(), "verify_refresh_token should reject access tokens");
    }

    #[test]
    fn test_api_key_length_check() {
        assert!(api_key_prefix("nw_ak_12345").is_none());
    }

    #[test]
    fn test_api_key_prefix_validation() {
        assert_eq!(api_key_prefix("nw_ak_12345678"), Some("nw_ak_12345678"));
    }

    #[test]
    fn test_api_key_prefix_rejects_non_ascii_without_panicking() {
        let tricky_key = "nw_ak_123456€";
        assert!(tricky_key.len() >= 14);
        assert!(api_key_prefix(tricky_key).is_none());
    }

    #[test]
    fn test_create_access_token_no_panic() {
        // Issue #20: Verify token creation doesn't panic on overflow
        // (real overflow would happen ~year 262144, but we verify the function returns Result)
        let secret = "test-secret";
        let token_result = create_access_token(Uuid::new_v4(), secret);
        assert!(token_result.is_ok(), "token creation should not panic, should return Result");
    }

    #[test]
    fn test_create_refresh_token_no_panic() {
        // Issue #20: Verify refresh token creation doesn't panic on overflow
        let secret = "test-secret";
        let token_result = create_refresh_token(Uuid::new_v4(), secret);
        assert!(token_result.is_ok(), "refresh token creation should not panic, should return Result");
    }
}
