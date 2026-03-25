use axum::{extract::State, http::StatusCode, Json};
use netwatch_core::types::{IngestRequest, IngestResponse};
use std::sync::Arc;
use tracing::info;

use crate::auth::AgentAuth;
use crate::AppState;

pub async fn ingest(
    agent: AgentAuth,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, StatusCode> {
    if payload.snapshots.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.snapshots.len() > 100 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let host_id = payload.host.host_id;

    // Upsert host
    sqlx::query(
        r#"
        INSERT INTO hosts (id, account_id, api_key_id, hostname, os, kernel, agent_version, uptime_secs, cpu_model, cpu_cores, memory_total_bytes, last_seen_at, is_online)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, now(), true)
        ON CONFLICT (id) DO UPDATE SET
            hostname = EXCLUDED.hostname,
            os = EXCLUDED.os,
            kernel = EXCLUDED.kernel,
            agent_version = EXCLUDED.agent_version,
            uptime_secs = EXCLUDED.uptime_secs,
            cpu_model = EXCLUDED.cpu_model,
            cpu_cores = EXCLUDED.cpu_cores,
            memory_total_bytes = EXCLUDED.memory_total_bytes,
            last_seen_at = now(),
            is_online = true
        "#,
    )
    .bind(host_id)
    .bind(agent.account_id)
    .bind(agent.api_key_id)
    .bind(&payload.host.hostname)
    .bind(&payload.host.os)
    .bind(&payload.host.kernel)
    .bind(&payload.agent_version)
    .bind(payload.host.uptime_secs.map(|v| v as i64))
    .bind(&payload.host.cpu_model)
    .bind(payload.host.cpu_cores.map(|v| v as i32))
    .bind(payload.host.memory_total_bytes.map(|v| v as i64))
    .execute(&state.db)
    .await
    .map_err(|e| {
        tracing::error!("failed to upsert host: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut accepted = 0u32;

    for snapshot in &payload.snapshots {
        // Insert snapshot
        let snapshot_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO snapshots (host_id, time, connection_count, gateway_ip, gateway_rtt_ms, gateway_loss_pct, dns_ip, dns_rtt_ms, dns_loss_pct, cpu_usage_pct, memory_total_bytes, memory_used_bytes, memory_available_bytes, load_avg_1m, load_avg_5m, load_avg_15m, swap_total_bytes, swap_used_bytes, disk_read_bytes, disk_write_bytes, tcp_time_wait, tcp_close_wait)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
            RETURNING id
            "#,
        )
        .bind(host_id)
        .bind(snapshot.timestamp)
        .bind(snapshot.connection_count.map(|v| v as i32))
        .bind(snapshot.health.as_ref().and_then(|h| h.gateway_ip.as_deref()))
        .bind(snapshot.health.as_ref().and_then(|h| h.gateway_rtt_ms))
        .bind(snapshot.health.as_ref().and_then(|h| h.gateway_loss_pct))
        .bind(snapshot.health.as_ref().and_then(|h| h.dns_ip.as_deref()))
        .bind(snapshot.health.as_ref().and_then(|h| h.dns_rtt_ms))
        .bind(snapshot.health.as_ref().and_then(|h| h.dns_loss_pct))
        .bind(snapshot.system.as_ref().and_then(|s| s.cpu_usage_pct))
        .bind(snapshot.system.as_ref().and_then(|s| s.memory_total_bytes.map(|v| v as i64)))
        .bind(snapshot.system.as_ref().and_then(|s| s.memory_used_bytes.map(|v| v as i64)))
        .bind(snapshot.system.as_ref().and_then(|s| s.memory_available_bytes.map(|v| v as i64)))
        .bind(snapshot.system.as_ref().and_then(|s| s.load_avg_1m))
        .bind(snapshot.system.as_ref().and_then(|s| s.load_avg_5m))
        .bind(snapshot.system.as_ref().and_then(|s| s.load_avg_15m))
        .bind(snapshot.system.as_ref().and_then(|s| s.swap_total_bytes.map(|v| v as i64)))
        .bind(snapshot.system.as_ref().and_then(|s| s.swap_used_bytes.map(|v| v as i64)))
        .bind(snapshot.disk_io.as_ref().map(|d| d.read_bytes as i64))
        .bind(snapshot.disk_io.as_ref().map(|d| d.write_bytes as i64))
        .bind(snapshot.tcp_time_wait.map(|v| v as i32))
        .bind(snapshot.tcp_close_wait.map(|v| v as i32))
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            tracing::error!("failed to insert snapshot: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Insert interface metrics
        for iface in &snapshot.interfaces {
            sqlx::query(
                r#"
                INSERT INTO interface_metrics (snapshot_id, host_id, time, name, is_up, rx_bytes_total, tx_bytes_total, rx_bytes_delta, tx_bytes_delta, rx_packets, tx_packets, rx_errors, tx_errors, rx_drops, tx_drops)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                "#,
            )
            .bind(snapshot_id)
            .bind(host_id)
            .bind(snapshot.timestamp)
            .bind(&iface.name)
            .bind(iface.is_up)
            .bind(iface.rx_bytes as i64)
            .bind(iface.tx_bytes as i64)
            .bind(iface.rx_bytes_delta as i64)
            .bind(iface.tx_bytes_delta as i64)
            .bind(iface.rx_packets as i64)
            .bind(iface.tx_packets as i64)
            .bind(iface.rx_errors as i64)
            .bind(iface.tx_errors as i64)
            .bind(iface.rx_drops as i64)
            .bind(iface.tx_drops as i64)
            .execute(&state.db)
            .await
            .map_err(|e| {
                tracing::error!("failed to insert interface metric: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }

        // Insert disk metrics
        if let Some(ref disks) = snapshot.disk_usage {
            for disk in disks {
                sqlx::query(
                    r#"
                    INSERT INTO disk_metrics (snapshot_id, host_id, time, mount_point, device, total_bytes, used_bytes, available_bytes, usage_pct)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                    "#,
                )
                .bind(snapshot_id)
                .bind(host_id)
                .bind(snapshot.timestamp)
                .bind(&disk.mount_point)
                .bind(&disk.device)
                .bind(disk.total_bytes as i64)
                .bind(disk.used_bytes as i64)
                .bind(disk.available_bytes as i64)
                .bind(disk.usage_pct)
                .execute(&state.db)
                .await
                .map_err(|e| {
                    tracing::error!("failed to insert disk metric: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
        }

        accepted += 1;
    }

    info!("ingested {} snapshots for host {}", accepted, host_id);

    Ok(Json(IngestResponse {
        accepted,
        host_id,
    }))
}
