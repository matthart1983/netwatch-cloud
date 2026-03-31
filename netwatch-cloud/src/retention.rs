use crate::AppState;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};
use uuid::Uuid;

pub async fn run(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(3600));

    info!("data retention cleanup job started");

    loop {
        ticker.tick().await;

        if let Err(e) = cleanup_cycle(&state).await {
            error!("retention cleanup error: {}", e);
        }
    }
}

async fn cleanup_cycle(state: &Arc<AppState>) -> anyhow::Result<()> {
    // Delete snapshots for expired accounts immediately
    let expired = sqlx::query("DELETE FROM snapshots WHERE host_id IN (SELECT id FROM hosts WHERE account_id IN (SELECT id FROM accounts WHERE plan = 'expired'))")
        .execute(&state.db)
        .await?;
    info!(
        "retention: deleted {} snapshots for expired accounts",
        expired.rows_affected()
    );

    // Clean up per-account snapshots based on retention_days
    let accounts = sqlx::query_as::<_, (Uuid, i32)>(
        "SELECT id, retention_days FROM accounts WHERE plan NOT IN ('expired')"
    )
    .fetch_all(&state.db)
    .await?;

    for (account_id, retention_days) in accounts {
        let snapshots = sqlx::query(
            "DELETE FROM snapshots WHERE host_id IN (SELECT id FROM hosts WHERE account_id = $1) AND time < now() - INTERVAL '1 days' * $2"
        )
        .bind(account_id)
        .bind(retention_days)
        .execute(&state.db)
        .await?;

        if snapshots.rows_affected() > 0 {
            info!(
                "retention: account {} ({}d): deleted {} snapshots",
                account_id,
                retention_days,
                snapshots.rows_affected()
            );
        }
    }

    // Delete alert events older than 30 days
    let events =
        sqlx::query("DELETE FROM alert_events WHERE created_at < now() - INTERVAL '30 days'")
            .execute(&state.db)
            .await?;
    info!(
        "retention: deleted {} old alert events",
        events.rows_affected()
    );

    // Mark hosts offline if no snapshot in 5 minutes
    let hosts = sqlx::query(
        "UPDATE hosts SET is_online = false WHERE last_seen_at < now() - INTERVAL '5 minutes' AND is_online = true",
    )
    .execute(&state.db)
    .await?;
    info!(
        "retention: marked {} hosts offline",
        hosts.rows_affected()
    );

    Ok(())
}
