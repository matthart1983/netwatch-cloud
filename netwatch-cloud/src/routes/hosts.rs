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

#[derive(Serialize)]
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
    pub disk_usage_pct: Option<f64>,
    pub tcp_time_wait: Option<i32>,
    pub tcp_close_wait: Option<i32>,
    pub net_rx_bytes: Option<i64>,
    pub net_tx_bytes: Option<i64>,
    pub cpu_per_core: Option<Vec<f64>>,
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
        SELECT time, gateway_rtt_ms, gateway_loss_pct, dns_rtt_ms, dns_loss_pct, connection_count, cpu_usage_pct, memory_used_bytes, memory_available_bytes, load_avg_1m, load_avg_5m, load_avg_15m, swap_total_bytes, swap_used_bytes, disk_read_bytes, disk_write_bytes, tcp_time_wait, tcp_close_wait, cpu_per_core
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
                disk_usage_pct: None,
                tcp_time_wait: row.get("tcp_time_wait"),
                tcp_close_wait: row.get("tcp_close_wait"),
                net_rx_bytes: None,
                net_tx_bytes: None,
                cpu_per_core: row.get("cpu_per_core"),
            }
        })
        .collect();

    // Fetch aggregated network utilisation per snapshot time
    let net_rows = sqlx::query(
        r#"
        SELECT time, SUM(rx_bytes_delta)::bigint as rx, SUM(tx_bytes_delta)::bigint as tx
        FROM interface_metrics
        WHERE host_id = $1 AND time >= $2 AND time <= $3
        GROUP BY time
        ORDER BY time ASC
        "#,
    )
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build a lookup map from time -> (rx, tx)
    let net_map: std::collections::HashMap<String, (i64, i64)> = net_rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            let time: DateTime<Utc> = row.get("time");
            let rx: Option<i64> = row.get("rx");
            let tx: Option<i64> = row.get("tx");
            (time.to_rfc3339(), (rx.unwrap_or(0), tx.unwrap_or(0)))
        })
        .collect();

    // Fetch max disk usage % per snapshot time (highest mount point)
    let disk_rows = sqlx::query(
        r#"
        SELECT time, MAX(usage_pct) as max_usage
        FROM disk_metrics
        WHERE host_id = $1 AND time >= $2 AND time <= $3
        GROUP BY time
        ORDER BY time ASC
        "#,
    )
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let disk_map: std::collections::HashMap<String, f64> = disk_rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            let time: DateTime<Utc> = row.get("time");
            let usage: Option<f64> = row.get("max_usage");
            (time.to_rfc3339(), usage.unwrap_or(0.0))
        })
        .collect();

    // Merge network + disk data into points
    let points: Vec<MetricPoint> = points
        .into_iter()
        .map(|mut p| {
            let key = p.time.to_rfc3339();
            if let Some(&(rx, tx)) = net_map.get(&key) {
                p.net_rx_bytes = Some(rx);
                p.net_tx_bytes = Some(tx);
            }
            if let Some(&usage) = disk_map.get(&key) {
                p.disk_usage_pct = Some(usage);
            }
            p
        })
        .collect();

    Ok(Json(MetricsResponse {
        host_id: id,
        from,
        to,
        points,
    }))
}

#[derive(Serialize)]
pub struct DiskMount {
    pub mount_point: String,
    pub device: String,
    pub total_bytes: i64,
    pub used_bytes: i64,
    pub available_bytes: i64,
    pub usage_pct: f64,
    pub time: DateTime<Utc>,
}

pub async fn get_disks(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<DiskMount>>, StatusCode> {
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

    // Get the latest snapshot's disk metrics
    let rows = sqlx::query(
        r#"
        SELECT mount_point, device, total_bytes, used_bytes, available_bytes, usage_pct, time
        FROM disk_metrics
        WHERE host_id = $1 AND time = (
            SELECT MAX(time) FROM disk_metrics WHERE host_id = $1
        )
        ORDER BY mount_point
        "#,
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let disks: Vec<DiskMount> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            DiskMount {
                mount_point: row.get("mount_point"),
                device: row.get("device"),
                total_bytes: row.get("total_bytes"),
                used_bytes: row.get("used_bytes"),
                available_bytes: row.get("available_bytes"),
                usage_pct: row.get("usage_pct"),
                time: row.get("time"),
            }
        })
        .collect();

    Ok(Json(disks))
}

#[derive(Serialize)]
pub struct InterfaceStat {
    pub name: String,
    pub is_up: bool,
    pub rx_bytes_total: i64,
    pub tx_bytes_total: i64,
    pub rx_bytes_delta: i64,
    pub tx_bytes_delta: i64,
    pub rx_packets: i64,
    pub tx_packets: i64,
    pub rx_errors: i64,
    pub tx_errors: i64,
    pub rx_drops: i64,
    pub tx_drops: i64,
    pub time: DateTime<Utc>,
}

pub async fn get_interfaces(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Query(query): Query<MetricsQuery>,
) -> Result<Json<Vec<InterfaceStat>>, StatusCode> {
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
        SELECT name, is_up, rx_bytes_total, tx_bytes_total, rx_bytes_delta, tx_bytes_delta,
               rx_packets, tx_packets, rx_errors, tx_errors, rx_drops, tx_drops, time
        FROM interface_metrics
        WHERE host_id = $1 AND time >= $2 AND time <= $3
        ORDER BY name, time ASC
        "#,
    )
    .bind(id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let interfaces: Vec<InterfaceStat> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            InterfaceStat {
                name: row.get("name"),
                is_up: row.get("is_up"),
                rx_bytes_total: row.get("rx_bytes_total"),
                tx_bytes_total: row.get("tx_bytes_total"),
                rx_bytes_delta: row.get("rx_bytes_delta"),
                tx_bytes_delta: row.get("tx_bytes_delta"),
                rx_packets: row.get("rx_packets"),
                tx_packets: row.get("tx_packets"),
                rx_errors: row.get("rx_errors"),
                tx_errors: row.get("tx_errors"),
                rx_drops: row.get("rx_drops"),
                tx_drops: row.get("tx_drops"),
                time: row.get("time"),
            }
        })
        .collect();

    Ok(Json(interfaces))
}
