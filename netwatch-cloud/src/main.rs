use anyhow::Result;
use axum::{middleware, routing::{get, post}, Router, response::Response, body::Body};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info;
use tokio::signal;

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

    // Issue #13: Use PostgreSQL advisory locks to prevent duplicate jobs in multi-instance setup
    // Spawn alert engine background task with advisory lock
    let alert_state = state.clone();
    tokio::spawn(async move {
        loop {
            // Try to acquire advisory lock for alert engine (lock ID: 1001)
            let locked = sqlx::query_scalar::<_, bool>(
                "SELECT pg_try_advisory_lock(1001)"
            )
            .fetch_one(&alert_state.db)
            .await
            .unwrap_or(false);

            if locked {
                info!("alert engine acquired lock, running cycle");
                alerts::engine::run(alert_state.clone()).await;
                
                // Release lock when engine exits
                let _ = sqlx::query("SELECT pg_advisory_unlock(1001)")
                    .execute(&alert_state.db)
                    .await;
            } else {
                info!("alert engine lock held by another instance, waiting");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    });

    // Spawn data retention cleanup job (runs hourly) with advisory lock
    let retention_state = state.clone();
    tokio::spawn(async move {
        loop {
            // Try to acquire advisory lock for retention job (lock ID: 1002)
            let locked = sqlx::query_scalar::<_, bool>(
                "SELECT pg_try_advisory_lock(1002)"
            )
            .fetch_one(&retention_state.db)
            .await
            .unwrap_or(false);

            if locked {
                info!("retention job acquired lock, running cycle");
                retention::run(retention_state.clone()).await;
                
                // Release lock when job exits
                let _ = sqlx::query("SELECT pg_advisory_unlock(1002)")
                    .execute(&retention_state.db)
                    .await;
            } else {
                info!("retention job lock held by another instance, waiting");
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        }
    });

    let limiter = rate_limit::RateLimiter::new();

    // Middleware function to add security headers
    async fn security_headers_middleware(
        request: axum::http::Request<Body>,
        next: middleware::Next,
    ) -> Response {
        let mut res = next.run(request).await;
        res.headers_mut().insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );
        res.headers_mut().insert(
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self'".parse().unwrap(),
        );
        res
    }

    let app = Router::new()
        .route("/health", get(routes::health::health_check))
        .route("/version", get(routes::health::version))
        .route("/install.sh", get(routes::install::install_script))
        .route("/api/v1/ingest", post(routes::ingest::ingest))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/auth/forgot-password", post(routes::auth::forgot_password))
        .route("/api/v1/auth/reset-password", post(routes::auth::reset_password))
        .route("/api/v1/hosts", get(routes::hosts::list_hosts))
        .route("/api/v1/hosts/{id}", get(routes::hosts::get_host).delete(routes::hosts::delete_host))
        .route("/api/v1/hosts/{id}/metrics", get(routes::hosts::get_metrics))
        .route("/api/v1/hosts/{id}/disks", get(routes::hosts::get_disks))
        .route("/api/v1/hosts/{id}/interfaces", get(routes::hosts::get_interfaces))
        .route("/api/v1/account/api-keys", get(routes::api_keys::list_keys).post(routes::api_keys::create_key))
        .route("/api/v1/account/api-keys/{id}", axum::routing::delete(routes::api_keys::delete_key))
        .route("/api/v1/alerts/rules", get(routes::alerts::list_rules).post(routes::alerts::create_rule))
        .route("/api/v1/alerts/rules/{id}", axum::routing::put(routes::alerts::update_rule).delete(routes::alerts::delete_rule))
        .route("/api/v1/alerts/history", get(routes::alerts::alert_history))
        .route("/api/v1/account", get(routes::billing::get_account).put(routes::billing::update_account))
        .route("/api/v1/account/billing", get(routes::billing::get_billing))
        .route("/api/v1/webhooks/stripe", post(routes::billing::stripe_webhook))
        // Issue #16: Add 5MB max request body limit to prevent DoS attacks
        .layer(RequestBodyLimitLayer::new(5_000_000))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(middleware::from_fn(rate_limit::rate_limit_middleware))
        .layer(axum::Extension(limiter))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("listening on {}", bind_addr);

    // Issue #15: Setup graceful shutdown on SIGTERM/Ctrl+C
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
    
    tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        info!("received shutdown signal, initiating graceful shutdown");
        let _ = shutdown_tx.send(());
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            info!("graceful shutdown complete");
        })
        .await?;

    Ok(())
}
