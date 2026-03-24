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
        INSERT INTO hosts (id, account_id, api_key_id, hostname, os, kernel, agent_version, uptime_secs, last_seen_at, is_online)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), true)
        ON CONFLICT (id) DO UPDATE SET
            hostname = EXCLUDED.hostname,
            os = EXCLUDED.os,
            kernel = EXCLUDED.kernel,
            agent_version = EXCLUDED.agent_version,
            uptime_secs = EXCLUDED.uptime_secs,
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
            INSERT INTO snapshots (host_id, time, connection_count, gateway_ip, gateway_rtt_ms, gateway_loss_pct, dns_ip, dns_rtt_ms, dns_loss_pct)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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

        accepted += 1;
    }

    info!("ingested {} snapshots for host {}", accepted, host_id);

    Ok(Json(IngestResponse {
        accepted,
        host_id,
    }))
}
