use anyhow::Result;
use axum::{middleware, routing::{get, post}, Router};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

mod alerts;
mod auth;
mod config;
mod rate_limit;
mod retention;
mod routes;

pub struct AppState {
    pub db: sqlx::PgPool,
    pub config: config::ServerConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let cfg = config::ServerConfig::from_env()?;
    let bind_addr = cfg.bind_addr.clone();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;

    info!("connected to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    info!("migrations applied");

    let state = Arc::new(AppState { db: pool, config: cfg });

    // Spawn alert engine background task
    let alert_state = state.clone();
    tokio::spawn(async move {
        alerts::engine::run(alert_state).await;
    });

    // Spawn data retention cleanup job (runs hourly)
    let retention_state = state.clone();
    tokio::spawn(async move {
        retention::run(retention_state).await;
    });

    let limiter = rate_limit::RateLimiter::new();

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/install.sh", get(routes::install::install_script))
        .route("/api/v1/ingest", post(routes::ingest::ingest))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/hosts", get(routes::hosts::list_hosts))
        .route("/api/v1/hosts/{id}", get(routes::hosts::get_host))
        .route("/api/v1/hosts/{id}/metrics", get(routes::hosts::get_metrics))
        .route("/api/v1/account/api-keys", get(routes::api_keys::list_keys).post(routes::api_keys::create_key))
        .route("/api/v1/account/api-keys/{id}", axum::routing::delete(routes::api_keys::delete_key))
        .route("/api/v1/alerts/rules", get(routes::alerts::list_rules).post(routes::alerts::create_rule))
        .route("/api/v1/alerts/rules/{id}", axum::routing::put(routes::alerts::update_rule).delete(routes::alerts::delete_rule))
        .route("/api/v1/alerts/history", get(routes::alerts::alert_history))
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(limiter))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("listening on {}", bind_addr);
    axum::serve(listener, app).await?;

    Ok(())
}
