use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

mod config;
mod collector;
mod host;
mod sender;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = config::AgentConfig::load()?;
    let host_id = host::get_or_create_host_id()?;
    let host_info = host::collect_host_info(host_id);

    info!(
        "netwatch-agent started, version {}, host_id={}",
        env!("CARGO_PKG_VERSION"),
        host_id
    );
    info!("endpoint: {}", cfg.endpoint);
    info!("interval: {}s, health interval: {}s", cfg.interval_secs, cfg.health_interval_secs);

    let mut collector = collector::MetricsCollector::new(&cfg);
    let mut sender = sender::Sender::new(&cfg, host_info);

    let interval = Duration::from_secs(cfg.interval_secs);
    let health_interval = Duration::from_secs(cfg.health_interval_secs);
    let mut last_health = tokio::time::Instant::now() - health_interval; // probe immediately

    loop {
        // Collect interface metrics every cycle
        let snapshot = collector.collect(last_health.elapsed() >= health_interval);

        if last_health.elapsed() >= health_interval {
            last_health = tokio::time::Instant::now();
        }

        match sender.send(snapshot) {
            Ok(()) => {
                info!("snapshot sent");
            }
            Err(e) => {
                warn!("send failed: {}, buffered ({} queued)", e, sender.buffer_len());
            }
        }

        tokio::time::sleep(interval).await;
    }
}
