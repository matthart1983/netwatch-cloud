use crate::AppState;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

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
    // Delete snapshots older than 72 hours (interface_metrics cascade via ON DELETE CASCADE)
    let snapshots = sqlx::query("DELETE FROM snapshots WHERE time < now() - INTERVAL '72 hours'")
        .execute(&state.db)
        .await?;
    info!(
        "retention: deleted {} old snapshots",
        snapshots.rows_affected()
    );

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
