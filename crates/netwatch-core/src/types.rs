use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub agent_version: String,
    pub host: HostInfo,
    pub snapshots: Vec<Snapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostInfo {
    pub host_id: Uuid,
    pub hostname: String,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub uptime_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub timestamp: DateTime<Utc>,
    pub interfaces: Vec<InterfaceMetric>,
    pub health: Option<HealthMetric>,
    pub connection_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceMetric {
    pub name: String,
    pub is_up: bool,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_bytes_delta: u64,
    pub tx_bytes_delta: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetric {
    pub gateway_ip: Option<String>,
    pub gateway_rtt_ms: Option<f64>,
    pub gateway_loss_pct: Option<f64>,
    pub dns_ip: Option<String>,
    pub dns_rtt_ms: Option<f64>,
    pub dns_loss_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub accepted: u32,
    pub host_id: Uuid,
}
