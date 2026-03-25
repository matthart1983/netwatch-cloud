use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, VecDeque<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        let limiter = Self {
            state: Arc::new(Mutex::new(HashMap::new())),
        };

        // Spawn sweep task to clear old entries every 10 minutes
        let sweep_state = limiter.state.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(600));
            loop {
                ticker.tick().await;
                let mut map = sweep_state.lock().unwrap();
                let cutoff = Instant::now();
                map.retain(|_, timestamps| {
                    timestamps.retain(|t| cutoff.duration_since(*t).as_secs() < 3600);
                    !timestamps.is_empty()
                });
            }
        });

        limiter
    }

    fn check(&self, key: &str, max_requests: u32, window_secs: u64) -> bool {
        let mut map = self.state.lock().unwrap();
        let timestamps = map.entry(key.to_string()).or_default();
        let now = Instant::now();

        // Remove timestamps outside the window
        while let Some(front) = timestamps.front() {
            if now.duration_since(*front).as_secs() >= window_secs {
                timestamps.pop_front();
            } else {
                break;
            }
        }

        if timestamps.len() >= max_requests as usize {
            return false;
        }

        timestamps.push_back(now);
        true
    }
}

pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Response {
    let limiter = request
        .extensions()
        .get::<RateLimiter>()
        .cloned();

    let Some(limiter) = limiter else {
        return next.run(request).await;
    };

    let path = request.uri().path().to_string();

    // Determine rate limit key and limits based on route
    let (key, max_requests, window_secs) = if path == "/api/v1/ingest" {
        // Agent ingest: keyed by Authorization header, 10/min
        let auth = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        (format!("ingest:{}", auth), 10, 60)
    } else if path == "/api/v1/auth/login" {
        // Login: keyed by IP, 5/min
        let ip = extract_ip(&request);
        (format!("login:{}", ip), 5, 60)
    } else if path == "/api/v1/auth/register" {
        // Registration: keyed by IP, 3/hour
        let ip = extract_ip(&request);
        (format!("register:{}", ip), 3, 3600)
    } else if path.starts_with("/api/v1/") {
        // Web API: keyed by Authorization header, 60/min
        let auth = request
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();
        (format!("api:{}", auth), 60, 60)
    } else {
        return next.run(request).await;
    };

    if !limiter.check(&key, max_requests, window_secs) {
        return (StatusCode::TOO_MANY_REQUESTS, "Too many requests").into_response();
    }

    next.run(request).await
}

fn extract_ip(request: &Request) -> String {
    request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}
