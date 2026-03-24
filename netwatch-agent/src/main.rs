use anyhow::Result;
use std::time::Duration;
use tracing::{info, warn};

mod collector;
mod config;
mod host;
mod sender;
mod update;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str());

    match cmd {
        Some("--version" | "-V" | "version") => {
            println!("netwatch-agent {}", VERSION);
            return Ok(());
        }
        Some("--help" | "-h" | "help") => {
            print_help();
            return Ok(());
        }
        Some("update") => {
            return update::self_update();
        }
        Some("status") => {
            return print_status();
        }
        Some("config") => {
            return print_config();
        }
        Some(unknown) if unknown.starts_with('-') || !unknown.is_empty() => {
            // Unknown commands starting with letters are errors
            if !unknown.starts_with('-') {
                eprintln!("Unknown command: {}", unknown);
                eprintln!("Run 'netwatch-agent help' for usage");
                std::process::exit(1);
            }
        }
        _ => {}
    }

    // Default: run the agent daemon
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

    info!("netwatch-agent started, version {}, host_id={}", VERSION, host_id);
    info!("endpoint: {}", cfg.endpoint);
    info!("interval: {}s, health interval: {}s", cfg.interval_secs, cfg.health_interval_secs);

    let mut collector = collector::MetricsCollector::new(&cfg);
    let mut sender = sender::Sender::new(&cfg, host_info);

    let interval = Duration::from_secs(cfg.interval_secs);
    let health_interval = Duration::from_secs(cfg.health_interval_secs);
    let mut last_health = tokio::time::Instant::now() - health_interval;

    loop {
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

fn print_help() {
    println!("netwatch-agent {} — network metrics collector", VERSION);
    println!();
    println!("USAGE:");
    println!("  netwatch-agent             Run the agent daemon");
    println!("  netwatch-agent update      Download and install the latest version");
    println!("  netwatch-agent status      Show agent status");
    println!("  netwatch-agent config      Show current configuration");
    println!("  netwatch-agent version     Print version");
    println!("  netwatch-agent help        Show this help");
    println!();
    println!("CONFIGURATION:");
    println!("  Config file: /etc/netwatch-agent/config.toml");
    println!("  Env vars:    NETWATCH_API_KEY, NETWATCH_ENDPOINT, NETWATCH_INTERVAL");
}

fn print_status() -> Result<()> {
    println!("netwatch-agent {}", VERSION);

    // Check if systemd service is running
    let output = std::process::Command::new("systemctl")
        .args(["is-active", "netwatch-agent"])
        .output();

    match output {
        Ok(o) => {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if status == "active" {
                println!("Status: ✅ running");
            } else {
                println!("Status: ❌ {}", status);
            }
        }
        Err(_) => {
            println!("Status: systemctl not available (running outside systemd?)");
        }
    }

    // Show host ID
    if let Ok(id) = std::fs::read_to_string("/var/lib/netwatch-agent/host-id") {
        println!("Host ID: {}", id.trim());
    }

    Ok(())
}

fn print_config() -> Result<()> {
    let cfg = config::AgentConfig::load()?;
    println!("Endpoint:  {}", cfg.endpoint);
    println!("API Key:   {}...", &cfg.api_key[..std::cmp::min(14, cfg.api_key.len())]);
    println!("Interval:  {}s", cfg.interval_secs);
    println!("Health:    {}s", cfg.health_interval_secs);
    if !cfg.interfaces.is_empty() {
        println!("Interfaces: {}", cfg.interfaces.join(", "));
    }
    if let Some(ref gw) = cfg.gateway {
        println!("Gateway:   {}", gw);
    }
    if let Some(ref dns) = cfg.dns_server {
        println!("DNS:       {}", dns);
    }
    Ok(())
}
