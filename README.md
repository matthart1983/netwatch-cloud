# NetWatch Cloud

Lightweight network monitoring for small Linux fleets. A companion cloud service for [NetWatch TUI](https://github.com/matthart1983/netwatch).

## Components

- **netwatch-agent** — Lightweight Linux daemon that collects 5 core metrics (interface status, latency, packet loss, connection count, heartbeat) and sends them to the cloud API
- **netwatch-cloud** — Axum-based API server (Rust) with PostgreSQL storage
- **web** — Next.js dashboard for viewing hosts, metrics, and managing alerts

## Quick Start

### 1. Run the API server

```bash
docker compose up -d                    # Start Postgres
cd netwatch-cloud && cargo run          # Start API server on :3001
```

### 2. Run the web dashboard

```bash
cd web && npm install && npm run dev    # Start frontend on :3000
```

### 3. Install the agent on a Linux server

```bash
curl -sSL https://install.netwatch.dev | sudo sh -s -- --api-key YOUR_KEY
```

Or build from source:

```bash
cargo build --release --package netwatch-agent
sudo cp target/release/netwatch-agent /usr/local/bin/
sudo mkdir -p /etc/netwatch-agent
# Create /etc/netwatch-agent/config.toml with your API key
sudo cp netwatch-agent/netwatch-agent.service /etc/systemd/system/
sudo systemctl enable --now netwatch-agent
```

## Architecture

```
Agent (Linux) ──── HTTPS POST JSON ────► API Server (Axum + Postgres)
                                              ▲
Web Dashboard (Next.js) ── REST API ──────────┘
```

## License

MIT
