use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub bind_addr: String,
    pub app_url: String,
    pub resend_api_key: Option<String>,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .context("DATABASE_URL must be set")?,
            jwt_secret: std::env::var("JWT_SECRET")
                .context("JWT_SECRET must be set")?,
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| {
                match std::env::var("PORT") {
                    Ok(port) => format!("0.0.0.0:{}", port),
                    Err(_) => "0.0.0.0:3001".to_string(),
                }
            }),
            app_url: std::env::var("APP_URL")
                .or_else(|_| std::env::var("NEXT_PUBLIC_APP_URL"))
                .unwrap_or_else(|_| "https://netwatch-web-production.up.railway.app".to_string()),
            resend_api_key: std::env::var("RESEND_API_KEY").ok(),
            stripe_secret_key: std::env::var("STRIPE_SECRET_KEY").ok(),
            stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
        })
    }
}
