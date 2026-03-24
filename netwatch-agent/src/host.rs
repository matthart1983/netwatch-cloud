use anyhow::Result;
use netwatch_core::collectors::system;
use netwatch_core::types::HostInfo;
use std::fs;
use std::path::Path;
use uuid::Uuid;

const HOST_ID_PATH: &str = "/var/lib/netwatch-agent/host-id";

pub fn get_or_create_host_id() -> Result<Uuid> {
    let path = Path::new(HOST_ID_PATH);

    if let Ok(contents) = fs::read_to_string(path) {
        if let Ok(id) = contents.trim().parse::<Uuid>() {
            return Ok(id);
        }
    }

    let id = Uuid::new_v4();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    // Best-effort write — if it fails (e.g. no write permission), we still return the ID
    // but it won't persist across restarts
    if fs::write(path, id.to_string()).is_err() {
        tracing::warn!("could not persist host-id to {}, using ephemeral ID", HOST_ID_PATH);
    }

    Ok(id)
}

pub fn collect_host_info(host_id: Uuid) -> HostInfo {
    // Env vars override auto-detection (useful when running in Docker)
    let hostname = std::env::var("NETWATCH_HOSTNAME").unwrap_or_else(|_| {
        fs::read_to_string("/proc/sys/kernel/hostname")
            .or_else(|_| fs::read_to_string("/etc/hostname"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    });

    let in_docker = Path::new("/.dockerenv").exists()
        || fs::read_to_string("/proc/1/cgroup")
            .map(|s| s.contains("docker") || s.contains("containerd"))
            .unwrap_or(false);

    let os = std::env::var("NETWATCH_OS").ok().or_else(|| {
        let detected = fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|contents| {
                contents
                    .lines()
                    .find(|l| l.starts_with("PRETTY_NAME="))
                    .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
            });
        match (detected, in_docker) {
            (Some(os), true) => Some(format!("{} (Docker)", os)),
            (Some(os), false) => Some(os),
            (None, true) => Some("Linux (Docker)".to_string()),
            (None, false) => None,
        }
    });

    let kernel = std::process::Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let uptime_secs = fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|v| v as u64);

    let cpu_info = system::detect_cpu_info();
    let memory_total = system::detect_memory_total();

    HostInfo {
        host_id,
        hostname,
        os,
        kernel,
        uptime_secs,
        cpu_model: cpu_info.model,
        cpu_cores: cpu_info.cores,
        memory_total_bytes: memory_total,
    }
}
