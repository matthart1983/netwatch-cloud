use crate::config::AgentConfig;
use netwatch_core::types::{HostInfo, IngestRequest, Snapshot};
use std::collections::VecDeque;

const MAX_BUFFER: usize = 100;

pub struct Sender {
    endpoint: String,
    api_key: String,
    host_info: HostInfo,
    buffer: VecDeque<Snapshot>,
    consecutive_failures: u32,
}

impl Sender {
    pub fn new(cfg: &AgentConfig, host_info: HostInfo) -> Self {
        Self {
            endpoint: cfg.endpoint.clone(),
            api_key: cfg.api_key.clone(),
            host_info,
            buffer: VecDeque::new(),
            consecutive_failures: 0,
        }
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn send(&mut self, snapshot: Snapshot) -> Result<(), String> {
        self.buffer.push_back(snapshot);

        // Drain buffer into a single request
        let snapshots: Vec<Snapshot> = self.buffer.drain(..).collect();

        let request = IngestRequest {
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            host: self.host_info.clone(),
            snapshots: snapshots.clone(),
        };

        let result = ureq::post(&self.endpoint)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .set("Content-Type", "application/json")
            .send_json(serde_json::to_value(&request).map_err(|e| e.to_string())?);

        match result {
            Ok(response) => {
                if response.status() == 200 {
                    self.consecutive_failures = 0;
                    Ok(())
                } else {
                    // Put snapshots back in buffer
                    for s in snapshots.into_iter().rev() {
                        self.buffer.push_front(s);
                    }
                    self.trim_buffer();
                    self.consecutive_failures += 1;
                    Err(format!("HTTP {}", response.status()))
                }
            }
            Err(e) => {
                // Put snapshots back in buffer
                for s in snapshots.into_iter().rev() {
                    self.buffer.push_front(s);
                }
                self.trim_buffer();
                self.consecutive_failures += 1;
                Err(e.to_string())
            }
        }
    }

    fn trim_buffer(&mut self) {
        while self.buffer.len() > MAX_BUFFER {
            self.buffer.pop_front();
        }
    }
}
