use crate::AppState;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use uuid::Uuid;

use super::notify;

#[derive(Debug, Clone, PartialEq)]
enum AlertState {
    Ok,
    Pending { since: Instant },
    Firing,
    Resolved,
}

type StateKey = (Uuid, Uuid); // (rule_id, host_id)

pub async fn run(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(30));
    let mut states: HashMap<StateKey, AlertState> = HashMap::new();

    info!("alert engine started");

    loop {
        ticker.tick().await;

        if let Err(e) = evaluate_cycle(&state, &mut states).await {
            error!("alert engine error: {}", e);
        }
    }
}

async fn evaluate_cycle(
    state: &Arc<AppState>,
    states: &mut HashMap<StateKey, AlertState>,
) -> anyhow::Result<()> {
    // Mark hosts offline
    sqlx::query(
        "UPDATE hosts SET is_online = false WHERE last_seen_at < now() - INTERVAL '5 minutes' AND is_online = true",
    )
    .execute(&state.db)
    .await?;

    // Load enabled rules
    let rules = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, String, String, Option<f64>, Option<String>, i32, String)>(
        "SELECT id, account_id, host_id, name, metric, condition, threshold, threshold_str, duration_secs, severity FROM alert_rules WHERE enabled = true",
    )
    .fetch_all(&state.db)
    .await?;

    for (rule_id, account_id, rule_host_id, name, metric, condition, threshold, threshold_str, duration_secs, severity) in &rules {
        // Get applicable hosts
        let hosts: Vec<(Uuid, String)> = if let Some(hid) = rule_host_id {
            sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, hostname FROM hosts WHERE id = $1 AND account_id = $2",
            )
            .bind(hid)
            .bind(account_id)
            .fetch_all(&state.db)
            .await?
        } else {
            sqlx::query_as::<_, (Uuid, String)>(
                "SELECT id, hostname FROM hosts WHERE account_id = $1",
            )
            .bind(account_id)
            .fetch_all(&state.db)
            .await?
        };

        for (host_id, hostname) in &hosts {
            let key = (*rule_id, *host_id);
            let current = states.get(&key).cloned().unwrap_or(AlertState::Ok);

            let (condition_met, metric_value) = check_condition(
                &state.db,
                *host_id,
                metric,
                condition,
                *threshold,
                threshold_str.as_deref(),
            )
            .await
            .unwrap_or((false, None));

            let new_state = match current {
                AlertState::Ok => {
                    if condition_met {
                        AlertState::Pending {
                            since: Instant::now(),
                        }
                    } else {
                        AlertState::Ok
                    }
                }
                AlertState::Pending { since } => {
                    if !condition_met {
                        AlertState::Ok
                    } else if since.elapsed().as_secs() >= *duration_secs as u64 {
                        // Fire!
                        let message = format!(
                            "{}: {} on host {}",
                            severity.to_uppercase(),
                            name,
                            hostname
                        );
                        let _ = sqlx::query(
                            "INSERT INTO alert_events (rule_id, host_id, state, metric_value, message) VALUES ($1, $2, 'firing', $3, $4)",
                        )
                        .bind(rule_id)
                        .bind(host_id)
                        .bind(metric_value)
                        .bind(&message)
                        .execute(&state.db)
                        .await;

                        notify::send_alert(
                            &state.db,
                            &state.config,
                            *account_id,
                            *rule_id,
                            *host_id,
                            severity,
                            &message,
                            hostname,
                        )
                        .await;

                        info!("ALERT FIRING: {}", message);
                        AlertState::Firing
                    } else {
                        AlertState::Pending { since }
                    }
                }
                AlertState::Firing => {
                    if !condition_met {
                        let message = format!("RESOLVED: {} on host {}", name, hostname);
                        let _ = sqlx::query(
                            "INSERT INTO alert_events (rule_id, host_id, state, metric_value, message) VALUES ($1, $2, 'resolved', $3, $4)",
                        )
                        .bind(rule_id)
                        .bind(host_id)
                        .bind(metric_value)
                        .bind(&message)
                        .execute(&state.db)
                        .await;

                        notify::send_alert(
                            &state.db,
                            &state.config,
                            *account_id,
                            *rule_id,
                            *host_id,
                            "resolved",
                            &message,
                            hostname,
                        )
                        .await;

                        info!("ALERT RESOLVED: {}", message);
                        AlertState::Resolved
                    } else {
                        AlertState::Firing
                    }
                }
                AlertState::Resolved => AlertState::Ok,
            };

            states.insert(key, new_state);
        }
    }

    Ok(())
}

async fn check_condition(
    db: &sqlx::PgPool,
    host_id: Uuid,
    metric: &str,
    condition: &str,
    threshold: Option<f64>,
    threshold_str: Option<&str>,
) -> anyhow::Result<(bool, Option<f64>)> {
    match metric {
        "host_status" => {
            let is_online: Option<bool> =
                sqlx::query_scalar("SELECT is_online FROM hosts WHERE id = $1")
                    .bind(host_id)
                    .fetch_optional(db)
                    .await?;

            let offline = is_online == Some(false);
            let met = condition == "changes_to" && threshold_str == Some("offline") && offline;
            Ok((met, if offline { Some(0.0) } else { Some(1.0) }))
        }
        "interface_status" => {
            let any_down: Option<bool> = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM interface_metrics im WHERE im.host_id = $1 AND im.is_up = false AND im.time > now() - INTERVAL '2 minutes')",
            )
            .bind(host_id)
            .fetch_one(db)
            .await?;

            let met = condition == "changes_to"
                && threshold_str == Some("down")
                && any_down == Some(true);
            Ok((met, None))
        }
        "disk_usage_pct" => {
            let value: Option<f64> = sqlx::query_scalar(
                "SELECT MAX(usage_pct) FROM disk_metrics WHERE host_id = $1 AND time > now() - INTERVAL '2 minutes'"
            )
            .bind(host_id)
            .fetch_optional(db)
            .await?
            .flatten();

            let Some(val) = value else {
                return Ok((false, None));
            };

            let Some(thresh) = threshold else {
                return Ok((false, Some(val)));
            };

            let met = match condition {
                ">" => val > thresh,
                "<" => val < thresh,
                "==" => (val - thresh).abs() < f64::EPSILON,
                _ => false,
            };

            Ok((met, Some(val)))
        }
        _ => {
            // Numeric metrics from snapshots
            let column = match metric {
                "gateway_rtt_ms" => "gateway_rtt_ms",
                "gateway_loss_pct" => "gateway_loss_pct",
                "dns_rtt_ms" => "dns_rtt_ms",
                "dns_loss_pct" => "dns_loss_pct",
                "connection_count" => "connection_count",
                "cpu_usage_pct" => "cpu_usage_pct",
                "swap_used_bytes" => "swap_used_bytes",
                "disk_read_bytes" => "disk_read_bytes",
                "disk_write_bytes" => "disk_write_bytes",
                "tcp_time_wait" => "tcp_time_wait",
                "tcp_close_wait" => "tcp_close_wait",
                _ => return Ok((false, None)),
            };

            let query = format!(
                "SELECT {}::double precision FROM snapshots WHERE host_id = $1 AND {} IS NOT NULL ORDER BY time DESC LIMIT 1",
                column, column
            );
            let value: Option<f64> = sqlx::query_scalar(&query)
                .bind(host_id)
                .fetch_optional(db)
                .await?;

            let Some(val) = value else {
                return Ok((false, None));
            };

            let Some(thresh) = threshold else {
                return Ok((false, Some(val)));
            };

            let met = match condition {
                ">" => val > thresh,
                "<" => val < thresh,
                "==" => (val - thresh).abs() < f64::EPSILON,
                _ => false,
            };

            Ok((met, Some(val)))
        }
    }
}
