use crate::config::AgentConfig;
use chrono::Utc;
use netwatch_core::collectors::{config as net_config, connections, health};
use netwatch_core::platform;
use netwatch_core::types::{HealthMetric, InterfaceMetric, Snapshot};
use std::collections::HashMap;

pub struct MetricsCollector {
    prev_bytes: HashMap<String, (u64, u64)>,
    gateway: Option<String>,
    dns_server: Option<String>,
    filter_interfaces: Vec<String>,
}

impl MetricsCollector {
    pub fn new(cfg: &AgentConfig) -> Self {
        let gateway = cfg.gateway.clone().or_else(|| net_config::detect_gateway());
        let dns_server = cfg.dns_server.clone().or_else(|| net_config::detect_dns());

        if let Some(ref gw) = gateway {
            tracing::info!("gateway: {}", gw);
        } else {
            tracing::warn!("no gateway detected");
        }
        if let Some(ref dns) = dns_server {
            tracing::info!("dns: {}", dns);
        } else {
            tracing::warn!("no dns server detected");
        }

        Self {
            prev_bytes: HashMap::new(),
            gateway,
            dns_server,
            filter_interfaces: cfg.interfaces.clone(),
        }
    }

    pub fn collect(&mut self, include_health: bool) -> Snapshot {
        let interfaces = self.collect_interfaces();
        let health = if include_health {
            Some(self.collect_health())
        } else {
            None
        };
        let connection_count = Some(connections::count_established_connections());

        Snapshot {
            timestamp: Utc::now(),
            interfaces,
            health,
            connection_count,
        }
    }

    fn collect_interfaces(&mut self) -> Vec<InterfaceMetric> {
        let stats = match platform::collect_interface_stats() {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("failed to collect interface stats: {}", e);
                return Vec::new();
            }
        };

        let mut metrics = Vec::new();

        for (name, stat) in &stats {
            if !self.filter_interfaces.is_empty()
                && !self.filter_interfaces.iter().any(|f| f == name)
            {
                continue;
            }

            let (rx_delta, tx_delta) = if let Some(&(prev_rx, prev_tx)) = self.prev_bytes.get(name) {
                (
                    stat.rx_bytes.saturating_sub(prev_rx),
                    stat.tx_bytes.saturating_sub(prev_tx),
                )
            } else {
                (0, 0)
            };

            self.prev_bytes.insert(name.clone(), (stat.rx_bytes, stat.tx_bytes));

            metrics.push(InterfaceMetric {
                name: name.clone(),
                is_up: stat.is_up,
                rx_bytes: stat.rx_bytes,
                tx_bytes: stat.tx_bytes,
                rx_bytes_delta: rx_delta,
                tx_bytes_delta: tx_delta,
                rx_packets: stat.rx_packets,
                tx_packets: stat.tx_packets,
                rx_errors: stat.rx_errors,
                tx_errors: stat.tx_errors,
                rx_drops: stat.rx_drops,
                tx_drops: stat.tx_drops,
            });
        }

        metrics.sort_by(|a, b| a.name.cmp(&b.name));
        metrics
    }

    fn collect_health(&self) -> HealthMetric {
        let (gateway_rtt, gateway_loss, gateway_ip) = if let Some(ref gw) = self.gateway {
            let result = health::run_ping(gw);
            (result.rtt_ms, Some(result.loss_pct), Some(gw.clone()))
        } else {
            (None, None, None)
        };

        let (dns_rtt, dns_loss, dns_ip) = if let Some(ref dns) = self.dns_server {
            let result = health::run_ping(dns);
            (result.rtt_ms, Some(result.loss_pct), Some(dns.clone()))
        } else {
            (None, None, None)
        };

        HealthMetric {
            gateway_ip,
            gateway_rtt_ms: gateway_rtt,
            gateway_loss_pct: gateway_loss,
            dns_ip,
            dns_rtt_ms: dns_rtt,
            dns_loss_pct: dns_loss,
        }
    }
}
