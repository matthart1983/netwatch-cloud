use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::AppState;

#[derive(Serialize)]
pub struct HostSummary {
    pub id: Uuid,
    pub hostname: String,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub agent_version: Option<String>,
    pub is_online: bool,
    pub last_seen_at: DateTime<Utc>,
    pub uptime_secs: Option<i64>,
    pub cpu_model: Option<String>,
    pub cpu_cores: Option<i32>,
    pub memory_total_bytes: Option<i64>,
}

pub async fn list_hosts(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<HostSummary>>, StatusCode> {
    let hosts = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>, Option<String>, bool, DateTime<Utc>, Option<i64>, Option<String>, Option<i32>, Option<i64>)>(
        "SELECT id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes FROM hosts WHERE account_id = $1 ORDER BY hostname"
    )
    .bind(user.account_id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<HostSummary> = hosts
        .into_iter()
        .map(|(id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes)| {
            HostSummary { id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes }
        })
        .collect();

    Ok(Json(result))
}

pub async fn get_host(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<HostSummary>, StatusCode> {
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>, Option<String>, bool, DateTime<Utc>, Option<i64>, Option<String>, Option<i32>, Option<i64>)>(
        "SELECT id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes FROM hosts WHERE id = $1 AND account_id = $2"
    )
    .bind(id)
    .bind(user.account_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let (id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes) = row;

    Ok(Json(HostSummary { id, hostname, os, kernel, agent_version, is_online, last_seen_at, uptime_secs, cpu_model, cpu_cores, memory_total_bytes }))
}

#[derive(Deserialize)]
pub struct MetricsQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct MetricsResponse {
    pub host_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub points: Vec<MetricPoint>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MetricPoint {
    pub time: DateTime<Utc>,
    pub gateway_rtt_ms: Option<f64>,
    pub gateway_loss_pct: Option<f64>,
    pub dns_rtt_ms: Option<f64>,
    pub dns_loss_pct: Option<f64>,
    pub connection_count: Option<i32>,
    pub cpu_usage_pct: Option<f64>,
    pub memory_used_bytes: Option<i64>,
    pub memory_available_bytes: Option<i64>,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
    pub swap_total_bytes: Option<i64>,
    pub swap_used_bytes: Option<i64>,
    pub disk_read_bytes: Option<i64>,
    pub disk_write_bytes: Option<i64>,
    pub tcp_time_wait: Option<i32>,
    pub tcp_close_wait: Option<i32>,
}

pub async fn get_metrics(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<MetricsResponse>, StatusCode> {
    // Verify host belongs to user
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM hosts WHERE id = $1 AND account_id = $2)"
    )
    .bind(id)
    .bind(user.account_id)
    .fetch_one(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }

    let now = Utc::now();
    let from = query.from.unwrap_or(now - chrono::Duration::hours(24));
    let to = query.to.unwrap_or(now);

    let rows = sqlx::query(
        r#"
        SELECT time, gateway_rtt_ms, gateway_loss_pct, dns_rtt_ms, dns_loss_pct, connection_count, cpu_usage_pct, memory_used_bytes, memory_available_bytes, load_avg_1m, load_avg_5m, load_avg_15m, swap_total_bytes, swap_used_bytes, disk_read_bytes, disk_write_bytes, tcp_time_wait, tcp_close_wait
        FROM snapshots
        WHERE host_id = $1 AND time >= $2 AND time <= $3
        ORDER BY time ASC
        "#,
    )
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let points: Vec<MetricPoint> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            MetricPoint {
                time: row.get("time"),
                gateway_rtt_ms: row.get("gateway_rtt_ms"),
                gateway_loss_pct: row.get("gateway_loss_pct"),
                dns_rtt_ms: row.get("dns_rtt_ms"),
                dns_loss_pct: row.get("dns_loss_pct"),
                connection_count: row.get("connection_count"),
                cpu_usage_pct: row.get("cpu_usage_pct"),
                memory_used_bytes: row.get("memory_used_bytes"),
                memory_available_bytes: row.get("memory_available_bytes"),
                load_avg_1m: row.get("load_avg_1m"),
                load_avg_5m: row.get("load_avg_5m"),
                load_avg_15m: row.get("load_avg_15m"),
                swap_total_bytes: row.get("swap_total_bytes"),
                swap_used_bytes: row.get("swap_used_bytes"),
                disk_read_bytes: row.get("disk_read_bytes"),
                disk_write_bytes: row.get("disk_write_bytes"),
                tcp_time_wait: row.get("tcp_time_wait"),
                tcp_close_wait: row.get("tcp_close_wait"),
            }
        })
        .collect();

    Ok(Json(MetricsResponse {
        host_id: id,
        from,
        to,
        points,
    }))
}
