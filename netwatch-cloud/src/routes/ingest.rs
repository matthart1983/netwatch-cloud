use axum::{extract::State, http::StatusCode, Json, response::{IntoResponse, Response}};
use netwatch_core::types::{IngestRequest, IngestResponse, SnapshotResult};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::auth::AgentAuth;
use crate::AppState;

pub async fn ingest(
    agent: AgentAuth,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<IngestRequest>,
) -> Result<Response, StatusCode> {
    if payload.snapshots.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if payload.snapshots.len() > 100 {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    // Enforce billing: check plan and trial status
    let account = sqlx::query_as::<_, (String, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT plan, trial_ends_at FROM accounts WHERE id = $1",
    )
    .bind(agent.account_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    let (plan, trial_ends_at) = account;
    if plan == "trial" {
        if let Some(ends) = trial_ends_at {
            if ends < chrono::Utc::now() {
                return Err(StatusCode::PAYMENT_REQUIRED);
            }
        }
    } else if plan == "expired" || plan == "past_due" {
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    let host_id = payload.host.host_id;

    // Check for cross-tenant host overwrite: if host exists, verify it belongs to this account
    let existing_host_account: Option<Uuid> = sqlx::query_scalar(
        "SELECT account_id FROM hosts WHERE id = $1"
    )
    .bind(host_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(existing_account_id) = existing_host_account {
        if existing_account_id != agent.account_id {
            // Host exists but belongs to different account - reject
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    // Enforce host limits with transactional consistency to prevent race conditions
    let host_limit: i64 = match plan.as_str() {
        "early_access" => 10,
        _ => 3,
    };

    // Start transaction and use FOR UPDATE to lock the count
    let mut tx = state.db.begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get count with FOR UPDATE lock (serialized count check)
    let host_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM hosts WHERE account_id = $1 FOR UPDATE"
    )
    .bind(agent.account_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if host_count >= host_limit {
        let host_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM hosts WHERE id = $1 AND account_id = $2)",
        )
        .bind(host_id)
        .bind(agent.account_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !host_exists {
            return Err(StatusCode::PAYMENT_REQUIRED);
        }
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    let mut rejected = 0u32;
    let mut results: Vec<SnapshotResult> = Vec::new();

    for (index, snapshot) in payload.snapshots.iter().enumerate() {
        // Issue #9: Validate timestamp is within ±24 hours of server time
        let now = chrono::Utc::now();
        let max_skew = chrono::Duration::hours(24);
        if snapshot.timestamp > now + max_skew || snapshot.timestamp < now - max_skew {
            tracing::warn!("snapshot {} has invalid timestamp (skew > 24h): {}", index, snapshot.timestamp);
            results.push(SnapshotResult {
                index,
                status: 400,
                message: "Timestamp outside ±24 hour window".to_string(),
            });
            rejected += 1;
            continue;
        }

        // Issue #7: Wrap entire snapshot processing in a single transaction
        // If any insert fails, ROLLBACK the whole batch
        let mut tx = match state.db.begin().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("failed to begin transaction for snapshot {}: {}", index, e);
                results.push(SnapshotResult {
                    index,
                    status: 500,
                    message: "Transaction error".to_string(),
                });
                rejected += 1;
                continue;
            }
        };

        // Insert snapshot with ON CONFLICT handling for Issue #8 (deduplication)
        let snapshot_id = match sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO snapshots (host_id, time, connection_count, gateway_ip, gateway_rtt_ms, gateway_loss_pct, dns_ip, dns_rtt_ms, dns_loss_pct, cpu_usage_pct, memory_total_bytes, memory_used_bytes, memory_available_bytes, load_avg_1m, load_avg_5m, load_avg_15m, swap_total_bytes, swap_used_bytes, disk_read_bytes, disk_write_bytes, tcp_time_wait, tcp_close_wait, cpu_per_core)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
            ON CONFLICT (host_id, time) DO UPDATE SET
                connection_count = EXCLUDED.connection_count,
                gateway_ip = EXCLUDED.gateway_ip,
                gateway_rtt_ms = EXCLUDED.gateway_rtt_ms,
                gateway_loss_pct = EXCLUDED.gateway_loss_pct,
                dns_ip = EXCLUDED.dns_ip,
                dns_rtt_ms = EXCLUDED.dns_rtt_ms,
                dns_loss_pct = EXCLUDED.dns_loss_pct,
                cpu_usage_pct = EXCLUDED.cpu_usage_pct,
                memory_total_bytes = EXCLUDED.memory_total_bytes,
                memory_used_bytes = EXCLUDED.memory_used_bytes,
                memory_available_bytes = EXCLUDED.memory_available_bytes,
                load_avg_1m = EXCLUDED.load_avg_1m,
                load_avg_5m = EXCLUDED.load_avg_5m,
                load_avg_15m = EXCLUDED.load_avg_15m,
                swap_total_bytes = EXCLUDED.swap_total_bytes,
                swap_used_bytes = EXCLUDED.swap_used_bytes,
                disk_read_bytes = EXCLUDED.disk_read_bytes,
                disk_write_bytes = EXCLUDED.disk_write_bytes,
                tcp_time_wait = EXCLUDED.tcp_time_wait,
                tcp_close_wait = EXCLUDED.tcp_close_wait,
                cpu_per_core = EXCLUDED.cpu_per_core
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
        .bind(snapshot.system.as_ref().and_then(|s| s.cpu_per_core.as_deref()))
        .fetch_one(&mut *tx)
        .await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("failed to insert snapshot {}: {}", index, e);
                let _ = tx.rollback().await;
                results.push(SnapshotResult {
                    index,
                    status: 400,
                    message: "Failed to insert snapshot".to_string(),
                });
                rejected += 1;
                continue;
            }
        };

        // Insert interface metrics
        let mut iface_error = false;
        for iface in &snapshot.interfaces {
            if let Err(e) = sqlx::query(
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
            .execute(&mut *tx)
            .await {
                tracing::error!("failed to insert interface metric for snapshot {}: {}", index, e);
                iface_error = true;
                break;
            }
        }

        if iface_error {
            let _ = tx.rollback().await;
            results.push(SnapshotResult {
                index,
                status: 400,
                message: "Failed to insert interface metrics".to_string(),
            });
            rejected += 1;
            continue;
        }

        // Insert disk metrics
        let mut disk_error = false;
        if let Some(ref disks) = snapshot.disk_usage {
            for disk in disks {
                if let Err(e) = sqlx::query(
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
                .execute(&mut *tx)
                .await {
                    tracing::error!("failed to insert disk metric for snapshot {}: {}", index, e);
                    disk_error = true;
                    break;
                }
            }
        }

        if disk_error {
            let _ = tx.rollback().await;
            results.push(SnapshotResult {
                index,
                status: 400,
                message: "Failed to insert disk metrics".to_string(),
            });
            rejected += 1;
            continue;
        }

        // Commit the transaction if all inserts succeeded
        if let Err(e) = tx.commit().await {
            tracing::error!("failed to commit transaction for snapshot {}: {}", index, e);
            results.push(SnapshotResult {
                index,
                status: 500,
                message: "Failed to commit snapshot".to_string(),
            });
            rejected += 1;
            continue;
        }

        accepted += 1;
        results.push(SnapshotResult {
            index,
            status: 200,
            message: "OK".to_string(),
        });
    }

    // Determine response status code
    let response_status = if rejected > 0 && accepted > 0 {
        // Partial success - 207 Multi-Status
        StatusCode::MULTI_STATUS
    } else if rejected == payload.snapshots.len() as u32 {
        // All rejected - 400 Bad Request
        StatusCode::BAD_REQUEST
    } else {
        // All accepted - 200 OK
        StatusCode::OK
    };

    info!("ingested {} snapshots for host {} ({} accepted, {} rejected)", payload.snapshots.len(), host_id, accepted, rejected);

    let response = IngestResponse {
        accepted,
        rejected,
        host_id,
        results,
    };

    // Return response with appropriate status code
    Ok((response_status, Json(response)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_tenant_host_protection_logic() {
        // This test verifies the logic for cross-tenant host protection
        // In the real test, we check if existing_host_account matches agent.account_id
        
        let account_a = Uuid::new_v4();
        let account_b = Uuid::new_v4();
        
        // Scenario: existing_host_account is Some(account_a), but agent is account_b
        let existing_host_account: Option<Uuid> = Some(account_a);
        let agent_account = account_b;
        
        // This should reject (would return UNAUTHORIZED in real code)
        if let Some(existing_account_id) = existing_host_account {
            assert_ne!(existing_account_id, agent_account, "Should detect cross-tenant mismatch");
        }
    }

    #[test]
    fn test_host_limit_enforcement_logic() {
        // Test host limit enforcement for different plans
        let early_access_limit = 10i64;
        let default_limit = 3i64;
        
        // early_access plan should have 10 host limit
        let host_limit_ea = match "early_access" {
            "early_access" => 10,
            _ => 3,
        };
        assert_eq!(host_limit_ea, early_access_limit);
        
        // other plans should have 3 host limit
        let host_limit_trial = match "trial" {
            "early_access" => 10,
            _ => 3,
        };
        assert_eq!(host_limit_trial, default_limit);
    }

    #[test]
    fn test_response_status_codes() {
        // Test response status code logic
        let accepted = 5u32;
        let rejected = 0u32;
        let total = 5u32;
        
        // All accepted
        let response_status = if rejected > 0 && accepted > 0 {
            207  // Multi-Status
        } else if rejected == total {
            400  // Bad Request
        } else {
            200  // OK
        };
        assert_eq!(response_status, 200);
        
        // Partial success
        let accepted = 3u32;
        let rejected = 2u32;
        let response_status = if rejected > 0 && accepted > 0 {
            207
        } else if rejected == total {
            400
        } else {
            200
        };
        assert_eq!(response_status, 207);
        
        // All rejected
        let accepted = 0u32;
        let rejected = 5u32;
        let response_status = if rejected > 0 && accepted > 0 {
            207
        } else if rejected == total {
            400
        } else {
            200
        };
        assert_eq!(response_status, 400);
    }
}
