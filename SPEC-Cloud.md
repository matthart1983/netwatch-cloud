# NetWatch Cloud — Technical Specification

**Scope:** MVP as defined in Roadmap-Adjusted.md — a lightweight Linux agent daemon, HTTPS ingest API, PostgreSQL storage, simple web dashboard with 24–72h history, and email + Slack alerting for core metrics.

**Audience:** Solo founder (Matt) building and shipping this in 4–8 weeks.

**Repository:** https://github.com/matthart1983/netwatch-cloud (private)

---

## Build Progress

| # | Item | Status | Notes |
|---|------|--------|-------|
| 1 | `netwatch-core` shared library | ✅ Done | Types, platform collectors, health, system metrics |
| 2 | `netwatch-agent` Linux daemon | ✅ Done | Collects CPU, memory, load avg, interfaces, health, connections |
| 3 | Agent self-update (`netwatch-agent update`) | ✅ Done | Downloads from GitHub releases, replaces binary, restarts systemd |
| 4 | Agent CLI (`status`, `config`, `help`, `version`) | ✅ Done | |
| 5 | Agent Docker image | ✅ Done | `Dockerfile.agent`, env var overrides for hostname/OS |
| 6 | `agent.sh` local manager script | ✅ Done | start/stop/update/logs/status for Docker-based agents |
| 7 | Install script (`install.sh`) | ✅ Done | `--api-key`, `--endpoint`, `--update`, `--remove` modes |
| 8 | Install script served from API (`/install.sh`) | ✅ Done | Embedded via `include_str!` |
| 9 | `netwatch-cloud` API server (Axum) | ✅ Done | All endpoints operational |
| 10 | Database schema + migrations | ✅ Done | 2 migrations (initial + system metrics) |
| 11 | Auth (register, login, JWT) | ✅ Done | bcrypt passwords, JWT access tokens |
| 12 | Agent auth (API keys) | ✅ Done | bcrypt-hashed keys, prefix-based lookup |
| 13 | Ingest endpoint | ✅ Done | Handles host upsert, snapshots, interface metrics, system metrics |
| 14 | Host list + detail API | ✅ Done | Includes CPU, memory, OS, kernel, uptime |
| 15 | Metrics query API | ✅ Done | Time range filtering, all metric types |
| 16 | API key management (CRUD) | ✅ Done | |
| 17 | Alert engine (background task) | ✅ Done | 30s eval loop, state machine, host offline detection |
| 18 | Alert rules CRUD API | ✅ Done | |
| 19 | Alert history API | ✅ Done | |
| 20 | Default alert rules on signup | ✅ Done | Host offline, packet loss, gateway/DNS latency |
| 21 | Slack notifications | ✅ Done | Webhook-based |
| 22 | Email notifications (Resend) | ✅ Done | |
| 23 | Web frontend (Next.js 16) | ✅ Done | Dark theme, Recharts, 7 pages |
| 24 | Login / Register pages | ✅ Done | Shows API key + install instructions on register |
| 25 | Host list (dashboard) page | ✅ Done | Cards with status, OS, cores, RAM |
| 26 | Host detail page with charts | ✅ Done | Latency, packet loss, connections, CPU, memory, load avg |
| 27 | Alerts page (rules + history) | ✅ Done | Create/toggle/delete rules, event history |
| 28 | Settings page | ✅ Done | API key management, install + agent command reference |
| 29 | CI (GitHub Actions) | ✅ Done | Check/test on push, cross-compile releases on tag |
| 30 | Railway deployment | ✅ Done | API + Web + Postgres, all live |
| 31 | Stripe billing | ❌ Not started | |
| 32 | Data retention cleanup job | ✅ Done | Hourly: snapshots 72h, alert_events 30d, host offline 5m |
| 33 | Rate limiting | ✅ Done | In-memory, per spec limits |
| 34 | Landing page | ✅ Done | |

### Live URLs

| Service | URL |
|---------|-----|
| API | https://netwatch-api-production.up.railway.app |
| Web | https://netwatch-web-production.up.railway.app |
| Health | https://netwatch-api-production.up.railway.app/health |
| Install script | https://netwatch-api-production.up.railway.app/install.sh |

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Agent Daemon (`netwatch-agent`)](#2-agent-daemon-netwatch-agent)
3. [API Server (`netwatch-cloud`)](#3-api-server-netwatch-cloud)
4. [Database Schema](#4-database-schema)
5. [Ingest Protocol](#5-ingest-protocol)
6. [Alert Engine](#6-alert-engine)
7. [Web Frontend](#7-web-frontend)
8. [Authentication & Security](#8-authentication--security)
9. [Deployment & Operations](#9-deployment--operations)
10. [Billing](#10-billing)
11. [Project Structure & Build](#11-project-structure--build)
12. [What Is Explicitly Out of Scope](#12-what-is-explicitly-out-of-scope)

---

## 1. System Overview

```
┌─────────────────────┐       HTTPS POST /api/v1/ingest
│  netwatch-agent     │──────────────────────────────────►┌──────────────────┐
│  (Linux daemon)     │       (JSON, every 15s)           │  netwatch-cloud  │
│                     │◄──────────────────────────────────│  (Axum server)   │
│  Collects 5 metrics │       200 OK / 401 / 429          │                  │
│  No TUI, no pcap    │                                   │  ┌────────────┐  │
│  systemd managed    │                                   │  │ PostgreSQL │  │
└─────────────────────┘                                   │  └────────────┘  │
                                                          │  ┌────────────┐  │
┌─────────────────────┐       HTTPS (browser)             │  │Alert Engine│  │
│  Web Dashboard      │◄─────────────────────────────────►│  └────────────┘  │
│  (Next.js app)      │       REST API + JWT auth         │                  │
└─────────────────────┘                                   └──────────────────┘
```

### Design Principles

1. **Boring technology.** Postgres, JSON, HTTPS, JWT. No Redis, no WebSocket, no message queues, no protobuf.
2. **One platform.** Linux only. No macOS/Windows agent.
3. **No root required.** The 5 core metrics are all readable without elevated privileges on Linux.
4. **Separate binaries.** The agent is a standalone Rust binary. It does not share code with the TUI at the binary level (it may share a `netwatch-core` library crate for collector logic).
5. **Ship in weeks, not months.** Every design decision optimizes for time-to-first-paying-customer.

---

## 2. Agent Daemon (`netwatch-agent`)

### 2.1 What It Collects

The agent collects exactly 5 metric groups. Nothing else.

| # | Metric | Source | Requires Root | Collection Interval |
|---|--------|--------|---------------|---------------------|
| 1 | **Interface status** (up/down per interface) | `/sys/class/net/*/operstate` | No | 15s |
| 2 | **Interface bandwidth** (RX/TX bytes, packets, errors, drops) | `/sys/class/net/*/statistics/` | No | 15s |
| 3 | **Packet loss** to gateway and primary DNS | `ping -c 3 -W 1 <target>` | No | 30s |
| 4 | **Latency** (RTT ms) to gateway and primary DNS | Same ping as above | No | 30s |
| 5 | **Connection count** (total established TCP connections) | `/proc/net/tcp` + `/proc/net/tcp6` | No | 15s |
| 6 | **CPU usage** (%) | `/proc/stat` (200ms sample) | No | 15s |
| 7 | **Memory** (total, used, available bytes) | `/proc/meminfo` | No | 15s |
| 8 | **Load average** (1m, 5m, 15m) | `/proc/loadavg` | No | 15s |
| 9 | **Heartbeat** (agent is alive, with host metadata) | Implicit in every POST | No | 15s |

Host metadata collected once on startup: hostname, OS, kernel, uptime, CPU model, CPU cores, total memory.

### 2.2 What It Reuses From NetWatch TUI

The agent reuses collector logic from the existing codebase, extracted into a shared `netwatch-core` crate:

| Existing Code | What Agent Uses |
|---------------|-----------------|
| `src/platform/linux.rs` → `collect_interface_stats()` | Interface byte counters, packet counts, errors, drops, up/down |
| `src/collectors/config.rs` → `collect_gateway()`, `collect_dns()` | Gateway IP and DNS server discovery |
| `src/collectors/health.rs` → `run_ping()`, `parse_loss()`, `parse_avg_rtt()` | Ping-based health probing |
| `src/collectors/connections.rs` → `parse_linux_connections()` | Connection counting (count only, not full enumeration) |

The agent does **not** use: `TrafficCollector` (has TUI-specific sparkline history), `PacketCollector`, `GeoCache`, `WhoisCache`, `InsightsCollector`, `TracerouteRunner`, or any `ui/` code.

### 2.3 Configuration

Single TOML file at `/etc/netwatch-agent/config.toml`:

```toml
# NetWatch Agent configuration

# Cloud API endpoint
endpoint = "https://api.netwatch.dev/api/v1/ingest"

# API key (issued during signup)
api_key = "nw_ak_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"

# Collection interval in seconds (minimum 10, default 15)
interval_secs = 15

# Health probe interval in seconds (minimum 15, default 30)
health_interval_secs = 30

# Interfaces to monitor (empty = all non-loopback interfaces)
# interfaces = ["eth0", "ens5"]

# Override gateway for health probes (auto-detected if empty)
# gateway = "192.168.1.1"

# Override DNS server for health probes (auto-detected if empty)
# dns_server = "8.8.8.8"
```

Environment variables override config file values:
- `NETWATCH_ENDPOINT` → `endpoint`
- `NETWATCH_API_KEY` → `api_key`
- `NETWATCH_INTERVAL` → `interval_secs`
- `NETWATCH_HOSTNAME` → override auto-detected hostname (useful in Docker)
- `NETWATCH_OS` → override auto-detected OS (useful in Docker)

### 2.4 Agent CLI

```
netwatch-agent              Run the agent daemon
netwatch-agent update       Download and install the latest version (requires sudo)
netwatch-agent status       Show version, service state, host ID
netwatch-agent config       Show current configuration
netwatch-agent version      Print version
netwatch-agent help         Show usage
```

The `update` subcommand downloads the latest binary from GitHub releases, replaces itself, and restarts the systemd service. Docker detection appends "(Docker)" to the OS string automatically.

### 2.5 Lifecycle

```
                  ┌──────────────┐
                  │  Start       │
                  │  Load config │
                  │  Detect host │
                  └──────┬───────┘
                         │
                         ▼
                  ┌──────────────┐
              ┌──►│  Sleep       │
              │   │  (interval)  │
              │   └──────┬───────┘
              │          │
              │          ▼
              │   ┌──────────────┐
              │   │  Collect     │
              │   │  5 metrics   │
              │   └──────┬───────┘
              │          │
              │          ▼
              │   ┌──────────────┐     ┌─────────────┐
              │   │  POST to     │────►│  Success?   │
              │   │  cloud API   │     └──────┬──────┘
              │   └──────────────┘            │
              │          │                    │
              │          ▼ yes               ▼ no
              │   ┌──────────────┐    ┌──────────────┐
              └───┤  Clear buf   │    │  Buffer      │
                  └──────────────┘    │  locally     │
                                      │  (up to 100  │
                                      │   snapshots) │
                                      └──────┬───────┘
                                             │
                                             └──► retry next cycle
                                                  with backoff
```

**Local buffering:** If the API is unreachable, the agent buffers up to 100 snapshots in memory (~25 minutes at 15s intervals). On reconnection, it sends buffered snapshots in a single batch POST. If the buffer fills, oldest snapshots are dropped. No disk persistence — if the agent restarts, the buffer is lost. This is acceptable for MVP.

**Backoff:** On consecutive failures, the agent doubles the retry delay: 15s → 30s → 60s → 120s → 120s (capped). Resets on success.

### 2.5 Host Identification

On first run, the agent generates a UUID v4 and writes it to `/var/lib/netwatch-agent/host-id`. This persists across restarts. If the file is deleted, a new ID is generated (the host appears as a new host in the dashboard).

Each snapshot includes host metadata:
- `host_id` (UUID)
- `hostname` (from `gethostname()`)
- `os` (from `/etc/os-release` → `PRETTY_NAME`)
- `kernel` (from `uname -r`)
- `agent_version` (compile-time constant)
- `uptime_secs` (from `/proc/uptime`)

### 2.6 Binary & Packaging

- **Binary name:** `netwatch-agent`
- **Target:** `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
- **Install method (MVP):** Download binary + copy systemd unit file. One-liner install script:

```bash
curl -sSL https://install.netwatch.dev | sh -s -- --api-key YOUR_KEY
```

The install script:
1. Downloads the correct binary for the architecture
2. Places it at `/usr/local/bin/netwatch-agent`
3. Creates `/etc/netwatch-agent/config.toml` with the provided API key
4. Installs and starts the systemd unit

### 2.7 systemd Unit

```ini
[Unit]
Description=NetWatch Agent — network metrics collector
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/netwatch-agent
Restart=always
RestartSec=5
User=netwatch
Group=netwatch
EnvironmentFile=-/etc/netwatch-agent/env

# Security hardening
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=/var/lib/netwatch-agent
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```

The agent runs as a dedicated `netwatch` user with minimal privileges. No root required.

### 2.8 Logging

Logs to stderr (captured by journald). Log levels: `error`, `warn`, `info`, `debug`. Default: `info`.

```
INFO  netwatch-agent started, version 0.1.0, host_id=abc123
INFO  collecting from 3 interfaces: eth0, ens5, docker0
INFO  gateway detected: 10.0.0.1, dns: 8.8.8.8
INFO  snapshot sent (248 bytes, 23ms)
WARN  API unreachable, buffering (3 snapshots queued)
INFO  buffer flushed (3 snapshots)
```

---

## 3. API Server (`netwatch-cloud`)

### 3.1 Technology

- **Framework:** Axum (Rust)
- **Database:** PostgreSQL 16 (plain Postgres, no extensions required for MVP)
- **Auth:** JWT (access tokens) + API keys (agent auth)
- **Deployment:** Single binary + Postgres. Docker Compose for dev, Railway for production.

### 3.2 API Endpoints

#### Agent Endpoints (API Key Auth)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/ingest` | Receive one or more metric snapshots |

#### Web Endpoints (JWT Auth)

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/auth/register` | Create account (email + password) |
| `POST` | `/api/v1/auth/login` | Login, returns JWT |
| `POST` | `/api/v1/auth/refresh` | Refresh JWT |
| `GET` | `/api/v1/hosts` | List all hosts for the account |
| `GET` | `/api/v1/hosts/:id` | Host detail + latest snapshot |
| `DELETE` | `/api/v1/hosts/:id` | Remove a host and its data |
| `GET` | `/api/v1/hosts/:id/metrics` | Historical metrics (query params: `metric`, `from`, `to`) |
| `POST` | `/api/v1/hosts/:id/api-key` | Regenerate API key for a host |
| `GET` | `/api/v1/alerts/rules` | List alert rules |
| `POST` | `/api/v1/alerts/rules` | Create alert rule |
| `PUT` | `/api/v1/alerts/rules/:id` | Update alert rule |
| `DELETE` | `/api/v1/alerts/rules/:id` | Delete alert rule |
| `GET` | `/api/v1/alerts/history` | Alert event history (query params: `host_id`, `from`, `to`, `state`) |
| `GET` | `/api/v1/account` | Account details |
| `PUT` | `/api/v1/account` | Update account (notification settings) |
| `GET` | `/api/v1/account/api-keys` | List all API keys |
| `POST` | `/api/v1/account/api-keys` | Create new API key (for a new host) |
| `DELETE` | `/api/v1/account/api-keys/:id` | Revoke an API key |

#### Public Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Server health check |

### 3.3 Ingest Endpoint Detail

`POST /api/v1/ingest`

**Headers:**
```
Authorization: Bearer nw_ak_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
Content-Type: application/json
```

**Request Body:**
```json
{
  "agent_version": "0.1.0",
  "host": {
    "host_id": "550e8400-e29b-41d4-a716-446655440000",
    "hostname": "web-prod-1",
    "os": "Ubuntu 24.04.1 LTS",
    "kernel": "6.8.0-45-generic",
    "uptime_secs": 864000
  },
  "snapshots": [
    {
      "timestamp": "2026-04-01T14:30:00Z",
      "interfaces": [
        {
          "name": "eth0",
          "is_up": true,
          "rx_bytes": 158293847562,
          "tx_bytes": 42938475623,
          "rx_bytes_delta": 1284756,
          "tx_bytes_delta": 384756,
          "rx_packets": 12847560,
          "tx_packets": 3847560,
          "rx_errors": 0,
          "tx_errors": 0,
          "rx_drops": 0,
          "tx_drops": 0
        }
      ],
      "health": {
        "gateway_ip": "10.0.0.1",
        "gateway_rtt_ms": 1.23,
        "gateway_loss_pct": 0.0,
        "dns_ip": "8.8.8.8",
        "dns_rtt_ms": 12.5,
        "dns_loss_pct": 0.0
      },
      "connection_count": 47
    }
  ]
}
```

The `snapshots` array normally contains 1 element. It contains multiple elements when the agent is flushing a buffer after reconnection. Maximum 100 snapshots per request.

**Response:**

| Status | Meaning |
|--------|---------|
| `200 OK` | All snapshots accepted |
| `207 Multi-Status` | Partial acceptance (some snapshots had validation errors) |
| `401 Unauthorized` | Invalid or missing API key |
| `413 Payload Too Large` | More than 100 snapshots or body > 256 KB |
| `429 Too Many Requests` | Rate limited (> 10 requests/minute per API key) |

**Response Body (200):**
```json
{
  "accepted": 1,
  "host_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

### 3.4 Metrics Query Endpoint Detail

`GET /api/v1/hosts/:id/metrics?metric=gateway_rtt_ms&from=2026-04-01T00:00:00Z&to=2026-04-01T23:59:59Z`

**Query Parameters:**

| Param | Required | Description |
|-------|----------|-------------|
| `metric` | No | Filter to specific metric. One of: `gateway_rtt_ms`, `gateway_loss_pct`, `dns_rtt_ms`, `dns_loss_pct`, `connection_count`, `interface_status`, `rx_bytes_delta`, `tx_bytes_delta` |
| `from` | No | Start time (ISO 8601). Default: 24h ago |
| `to` | No | End time (ISO 8601). Default: now |
| `interface` | No | Filter interface metrics to a specific interface name |

**Response Body:**
```json
{
  "host_id": "550e8400-e29b-41d4-a716-446655440000",
  "from": "2026-04-01T00:00:00Z",
  "to": "2026-04-01T23:59:59Z",
  "points": [
    {
      "time": "2026-04-01T00:00:15Z",
      "gateway_rtt_ms": 1.2,
      "gateway_loss_pct": 0.0,
      "dns_rtt_ms": 12.5,
      "dns_loss_pct": 0.0,
      "connection_count": 42,
      "interfaces": [
        {
          "name": "eth0",
          "is_up": true,
          "rx_bytes_delta": 128475,
          "tx_bytes_delta": 38475
        }
      ]
    }
  ]
}
```

**Downsampling:** For queries spanning > 6 hours, the server averages data points into 1-minute buckets. For > 24 hours, 5-minute buckets. For > 72 hours, 15-minute buckets. This is done with a simple SQL `date_trunc` + `AVG` group-by.

### 3.5 Rate Limiting

Simple in-memory rate limiter (no Redis needed at MVP scale):

| Resource | Limit |
|----------|-------|
| Agent ingest per API key | 10 requests/minute |
| Web API per JWT | 60 requests/minute |
| Login attempts per IP | 5/minute |
| Registration per IP | 3/hour |

Implemented with a `HashMap<String, VecDeque<Instant>>` behind a `Mutex`. Cleared on a 10-minute sweep timer. At MVP scale (< 100 hosts), this is perfectly adequate.

---

## 4. Database Schema

Plain PostgreSQL. No extensions required.

```sql
-- ─── Accounts ───────────────────────────────────────────────

CREATE TABLE accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Notification settings
    notify_email    BOOLEAN NOT NULL DEFAULT true,
    slack_webhook   TEXT,             -- nullable, user provides their Slack webhook URL

    -- Billing
    stripe_customer_id  TEXT,
    plan                TEXT NOT NULL DEFAULT 'trial',  -- 'trial', 'early_access'
    trial_ends_at       TIMESTAMPTZ
);

-- ─── API Keys ───────────────────────────────────────────────

CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id      UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_hash        TEXT NOT NULL,     -- bcrypt hash of the full key
    key_prefix      TEXT NOT NULL,     -- first 8 chars, for display: "nw_ak_ab..."
    label           TEXT,              -- user-provided label, e.g. "web-prod-1"
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_account ON api_keys(account_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);

-- ─── Hosts ──────────────────────────────────────────────────

CREATE TABLE hosts (
    id              UUID PRIMARY KEY,  -- matches agent-generated host_id
    account_id      UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    api_key_id      UUID NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    hostname        TEXT NOT NULL,
    os              TEXT,
    kernel          TEXT,
    agent_version   TEXT,
    uptime_secs     BIGINT,
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    is_online       BOOLEAN NOT NULL DEFAULT true
);

CREATE INDEX idx_hosts_account ON hosts(account_id);

-- ─── Metric Snapshots ───────────────────────────────────────

CREATE TABLE snapshots (
    id              BIGSERIAL PRIMARY KEY,
    host_id         UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    time            TIMESTAMPTZ NOT NULL,
    connection_count INTEGER,

    -- Health probes (nullable — probes may fail)
    gateway_ip      TEXT,
    gateway_rtt_ms  DOUBLE PRECISION,
    gateway_loss_pct DOUBLE PRECISION,
    dns_ip          TEXT,
    dns_rtt_ms      DOUBLE PRECISION,
    dns_loss_pct    DOUBLE PRECISION
);

CREATE INDEX idx_snapshots_host_time ON snapshots(host_id, time DESC);

-- ─── Interface Metrics (per-snapshot, per-interface) ────────

CREATE TABLE interface_metrics (
    id              BIGSERIAL PRIMARY KEY,
    snapshot_id     BIGINT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    host_id         UUID NOT NULL,     -- denormalized for faster queries
    time            TIMESTAMPTZ NOT NULL, -- denormalized
    name            TEXT NOT NULL,
    is_up           BOOLEAN NOT NULL,
    rx_bytes_total  BIGINT,
    tx_bytes_total  BIGINT,
    rx_bytes_delta  BIGINT,
    tx_bytes_delta  BIGINT,
    rx_packets      BIGINT,
    tx_packets      BIGINT,
    rx_errors       BIGINT,
    tx_errors       BIGINT,
    rx_drops        BIGINT,
    tx_drops        BIGINT
);

CREATE INDEX idx_iface_host_time ON interface_metrics(host_id, time DESC);
CREATE INDEX idx_iface_snapshot ON interface_metrics(snapshot_id);

-- ─── Alert Rules ────────────────────────────────────────────

CREATE TABLE alert_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id      UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    host_id         UUID REFERENCES hosts(id) ON DELETE CASCADE,  -- NULL = all hosts
    name            TEXT NOT NULL,
    metric          TEXT NOT NULL,       -- enum: see below
    condition       TEXT NOT NULL,       -- '>', '<', '==', 'changes_to'
    threshold       DOUBLE PRECISION,    -- numeric threshold (NULL for changes_to)
    threshold_str   TEXT,                -- string threshold for changes_to (e.g., 'down')
    duration_secs   INTEGER NOT NULL DEFAULT 60,  -- condition must hold for this long
    severity        TEXT NOT NULL DEFAULT 'warning', -- 'info', 'warning', 'critical'
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Metric enum values:
--   'gateway_rtt_ms'
--   'gateway_loss_pct'
--   'dns_rtt_ms'
--   'dns_loss_pct'
--   'connection_count'
--   'interface_status'   (uses threshold_str: 'down')
--   'host_status'        (uses threshold_str: 'offline')

CREATE INDEX idx_alert_rules_account ON alert_rules(account_id);

-- ─── Alert Events ───────────────────────────────────────────

CREATE TABLE alert_events (
    id              BIGSERIAL PRIMARY KEY,
    rule_id         UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    host_id         UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    state           TEXT NOT NULL,       -- 'firing', 'resolved'
    metric_value    DOUBLE PRECISION,
    message         TEXT NOT NULL,       -- human-readable: "Gateway latency 250ms > 100ms threshold"
    notified        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_alert_events_rule ON alert_events(rule_id, created_at DESC);
CREATE INDEX idx_alert_events_host ON alert_events(host_id, created_at DESC);
```

### 4.1 Data Retention

A cron job (or background task in the Axum server) runs hourly:

```sql
-- Delete snapshots older than 72 hours
DELETE FROM snapshots WHERE time < now() - INTERVAL '72 hours';

-- Delete interface metrics for deleted snapshots (cascaded)
-- Already handled by ON DELETE CASCADE

-- Delete alert events older than 30 days
DELETE FROM alert_events WHERE created_at < now() - INTERVAL '30 days';

-- Mark hosts as offline if no snapshot in 5 minutes
UPDATE hosts SET is_online = false
WHERE last_seen_at < now() - INTERVAL '5 minutes' AND is_online = true;
```

### 4.2 Storage Estimates

Per host at 15s intervals:
- Snapshots: 5,760 rows/day × ~200 bytes = ~1.1 MB/day
- Interface metrics: 5,760 × 3 interfaces × ~150 bytes = ~2.6 MB/day
- Total per host: **~3.7 MB/day**, **~267 MB for 72h retention**

At 100 hosts: ~26 GB for 72h. Comfortable on a $20/mo managed Postgres instance.

---

## 5. Ingest Protocol

### 5.1 Flow

```
Agent                                         Server
  │                                              │
  │── POST /api/v1/ingest ──────────────────────►│
  │   Authorization: Bearer nw_ak_xxxxx          │
  │   Content-Type: application/json             │
  │   Body: { host: {...}, snapshots: [...] }    │
  │                                              │
  │                            ┌─────────────────┤
  │                            │ 1. Verify key   │
  │                            │ 2. Upsert host  │
  │                            │ 3. Insert snap  │
  │                            │ 4. Check alerts │
  │                            └─────────────────┤
  │                                              │
  │◄── 200 OK ───────────────────────────────────│
  │    { "accepted": 1 }                         │
```

### 5.2 API Key Format

```
nw_ak_<32 random alphanumeric chars>

Example: nw_ak_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
```

- Generated server-side when user creates an API key in the dashboard
- Shown once to the user, then stored as a bcrypt hash
- The `key_prefix` (`nw_ak_a1b2c3d4`) is stored in plaintext for display and lookup

### 5.3 API Key Lookup (Fast Path)

To avoid bcrypt verification on every ingest request (4 requests/minute × N hosts):

1. On first successful verification, cache `key_hash → (account_id, api_key_id)` in an in-memory `HashMap`
2. Cache entries expire after 5 minutes
3. On cache miss, query `api_keys` table by `key_prefix`, then bcrypt-verify

This keeps ingest latency under 5ms for cached keys.

---

## 6. Alert Engine

### 6.1 Architecture

The alert engine runs as a background task inside the Axum server process. No separate service.

```
┌─────────────────────────────────────────────────────┐
│                  Axum Server Process                 │
│                                                     │
│  ┌──────────────┐    ┌───────────────────────────┐  │
│  │ HTTP Handler │    │    Alert Engine Task       │  │
│  │ (ingest)     │───►│    (runs every 30s)        │  │
│  └──────────────┘    │                           │  │
│                      │  1. Load enabled rules     │  │
│                      │  2. Query latest metrics   │  │
│                      │  3. Evaluate conditions    │  │
│                      │  4. Manage state machine   │  │
│                      │  5. Send notifications     │  │
│                      └───────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 6.2 State Machine (Per Rule × Per Host)

```
         condition met
    ┌───────────────────┐
    │                   ▼
 ┌──────┐         ┌──────────┐        duration elapsed        ┌─────────┐
 │  OK  │         │ PENDING  │───────────────────────────────►│ FIRING  │
 └──────┘         └──────────┘                                └─────────┘
    ▲                   │                                         │
    │   condition       │ condition                    condition  │
    │   not met         │ not met                      not met    │
    │                   ▼                                         │
    └───────────────────┘                                         │
    ▲                                                             │
    │                      ┌──────────┐                           │
    └──────────────────────│ RESOLVED │◄──────────────────────────┘
                           └──────────┘
```

States are stored in-memory (`HashMap<(rule_id, host_id), AlertState>`). On server restart, all states reset to OK. This is acceptable for MVP — the worst case is a re-notification for an ongoing alert.

### 6.3 Evaluation Logic

```rust
struct AlertState {
    state: State,              // OK, Pending, Firing, Resolved
    pending_since: Option<Instant>,
    last_notified: Option<Instant>,
}

enum State { Ok, Pending, Firing, Resolved }
```

Every 30 seconds, for each enabled rule:

1. **Query:** Get the latest metric value for each applicable host
2. **Evaluate:** Check if condition is met (e.g., `gateway_loss_pct > 5.0`)
3. **Transition:**
   - OK + condition met → PENDING (record `pending_since`)
   - PENDING + condition still met + `duration_secs` elapsed → FIRING (send notification)
   - PENDING + condition not met → OK
   - FIRING + condition not met → RESOLVED (send resolution notification)
   - RESOLVED → OK (on next cycle)

### 6.4 Default Alert Rules

Created automatically for every new account:

| Rule | Metric | Condition | Threshold | Duration | Severity |
|------|--------|-----------|-----------|----------|----------|
| Host offline | `host_status` | `changes_to` | `offline` | 60s | critical |
| Interface down | `interface_status` | `changes_to` | `down` | 60s | warning |
| High packet loss (gateway) | `gateway_loss_pct` | `>` | `5.0` | 60s | warning |
| High gateway latency | `gateway_rtt_ms` | `>` | `100.0` | 60s | warning |
| High DNS latency | `dns_rtt_ms` | `>` | `200.0` | 60s | info |

### 6.5 Notification Channels

#### Email

Uses Resend API (or Postmark). Simple transactional email:

**Subject:** `🔴 [CRITICAL] Host web-prod-1 is offline`

**Body:**
```
NetWatch Alert

Host: web-prod-1
Rule: Host offline
Status: FIRING
Time: 2026-04-01 14:30:00 UTC

The host has not reported in for 60 seconds.

View in dashboard: https://app.netwatch.dev/hosts/550e8400-...

---
Manage alerts: https://app.netwatch.dev/alerts
```

#### Slack

POST to user-provided incoming webhook URL:

```json
{
  "text": "🔴 *CRITICAL* — Host *web-prod-1* is offline",
  "blocks": [
    {
      "type": "section",
      "text": {
        "type": "mrkdwn",
        "text": "🔴 *CRITICAL* — Host *web-prod-1* is offline\n\n*Rule:* Host offline\n*Time:* 2026-04-01 14:30:00 UTC\n*Duration:* 60s\n\n<https://app.netwatch.dev/hosts/550e8400-...|View in dashboard>"
      }
    }
  ]
}
```

#### Rate Limiting Notifications

- Same alert does not re-notify more than once per 15 minutes while in FIRING state
- Resolution notifications are always sent immediately
- Maximum 50 notifications per account per hour

---

## 7. Web Frontend

### 7.1 Technology

- **Framework:** Next.js 15 (App Router)
- **Styling:** Tailwind CSS
- **Components:** shadcn/ui
- **Charts:** Recharts (one library, no alternatives)
- **Auth:** JWT stored in httpOnly cookie
- **Hosting:** Railway (same project as API server)

### 7.2 Pages

#### `/login` — Login Page
- Email + password form
- Link to registration

#### `/register` — Registration Page
- Email + password form
- Creates account + first API key
- Shows API key once with copy button and install command

#### `/` — Host List (Dashboard)
- Grid of host cards, each showing:
  - Hostname
  - Status indicator (green dot = online, red = offline, gray = never connected)
  - Last seen timestamp
  - OS + kernel
  - Gateway latency (latest)
  - Packet loss (latest)
  - Connection count (latest)
  - Interface status badges (up/down)
- "Add Host" button → shows API key + install command
- Empty state: clear setup instructions with install command

#### `/hosts/:id` — Host Detail
- **Header:** Hostname, status, OS, uptime, last seen, agent version
- **Health panel:** Gateway + DNS latency and packet loss, current values with 24h sparklines
- **Bandwidth chart:** RX/TX bytes over time (Recharts area chart), 24h default, 72h selectable
- **Interfaces table:** Name, status, current RX/TX rates, errors, drops
- **Connection count chart:** Line chart over time
- **Time range selector:** 1h / 6h / 24h / 72h buttons
- **Alert history for this host:** Recent alert events (last 20)

#### `/alerts` — Alert Rules
- List of alert rules with enable/disable toggle
- Create/edit rule form:
  - Name
  - Host (dropdown, or "All hosts")
  - Metric (dropdown)
  - Condition (dropdown: >, <, changes_to)
  - Threshold (number input or dropdown for status values)
  - Duration (seconds)
  - Severity (dropdown)
- Alert history tab: chronological list of alert events with filters

#### `/settings` — Account Settings
- **Profile:** Email (read-only for MVP)
- **Notifications:** Toggle email notifications, Slack webhook URL input
- **API Keys:** List of keys with labels, created date, last used. Create / revoke buttons.
- **Billing:** Link to Stripe Customer Portal (or "Trial: X days remaining")

### 7.3 Design

- Dark theme by default (matches TUI aesthetic)
- Minimal, data-dense layout — no hero images, no marketing fluff
- Mobile-responsive but optimized for desktop
- Real-time updates via polling (fetch every 15s on host detail page, every 60s on host list). No WebSocket.

---

## 8. Authentication & Security

### 8.1 User Authentication (Web)

- **Registration:** Email + password (minimum 8 chars). Password hashed with `argon2`.
- **Login:** Returns a JWT access token (30 min expiry) + refresh token (7 day expiry, stored in httpOnly cookie).
- **JWT payload:** `{ sub: account_id, exp: timestamp }`
- **JWT signing:** HMAC-SHA256 with a server-side secret from environment variable

### 8.2 Agent Authentication

- API keys are prefixed (`nw_ak_`) and hashed with bcrypt on the server
- Each API key is scoped to one account
- A key can serve multiple hosts (one key per account is fine for MVP)
- Keys can be revoked from the settings page

### 8.3 Security Measures

| Concern | Approach |
|---------|----------|
| Passwords | argon2 hashing |
| HTTPS | Required everywhere. HTTP redirects to HTTPS. HSTS header. |
| API keys in transit | Only over TLS. Agent refuses to send to non-HTTPS endpoints. |
| SQL injection | Parameterized queries via `sqlx` |
| XSS | React auto-escapes. CSP headers. |
| CSRF | JWT in httpOnly cookie + SameSite=Strict |
| Rate limiting | In-memory per-IP and per-key limits |
| Data isolation | All queries include `WHERE account_id = $1` |
| Secrets in code | All secrets via environment variables. `.env` in `.gitignore` |
| Dependency audit | `cargo audit` + `npm audit` in CI |

### 8.4 Data Privacy

The agent sends only:
- Interface names and byte counters
- Gateway/DNS IP addresses and latency
- Connection count (integer, no connection details)
- Hostname, OS, kernel version

It does **not** send:
- Packet contents
- Connection details (IPs, ports, process names)
- DNS queries
- User data
- File system information

This is a minimal, non-sensitive data set. The privacy story is simple and defensible.

---

## 9. Deployment & Operations

### 9.1 Production Stack

```
┌─────────────────────────────────────────────┐
│               Railway                        │
│                                             │
│  ┌──────────────────┐  ┌────────────────┐   │
│  │  netwatch-cloud  │  │  PostgreSQL    │   │
│  │  (Axum service)  │  │  (Railway DB)  │   │
│  └──────────────────┘  └────────────────┘   │
│                                             │
│  ┌──────────────────┐                       │
│  │  Next.js app     │                       │
│  │  (Railway svc)   │                       │
│  └──────────────────┘                       │
└─────────────────────────────────────────────┘
```

**Cost at launch:**
- Railway Hobby plan: $5/mo + usage (~$5–10/mo for light traffic)
- Railway Postgres (1 GB): included in usage pricing
- Domain: ~$12/year
- Resend (free tier, 3,000 emails/mo): $0/mo
- **Total: ~$10–15/mo**

### 9.2 Docker Compose (Development)

```yaml
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_DB: netwatch
      POSTGRES_USER: netwatch
      POSTGRES_PASSWORD: dev
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d

  api:
    build:
      context: ./netwatch-cloud
    environment:
      DATABASE_URL: postgres://netwatch:dev@db:5432/netwatch
      JWT_SECRET: dev-secret-change-me
      RESEND_API_KEY: ""
    ports:
      - "3001:3001"
    depends_on:
      - db

volumes:
  pgdata:
```

### 9.3 Database Migrations

Use `sqlx` CLI for migrations. Simple numbered SQL files:

```
migrations/
├── 001_accounts.sql
├── 002_api_keys.sql
├── 003_hosts.sql
├── 004_snapshots.sql
├── 005_interface_metrics.sql
├── 006_alert_rules.sql
└── 007_alert_events.sql
```

### 9.4 Monitoring

- **Server health:** `/health` endpoint returns 200 + database connectivity check
- **Agent connectivity:** `hosts.last_seen_at` — if > 5 min ago, host marked offline
- **Errors:** Log to stderr, captured by Railway logs. Add Sentry later if needed.
- **Alerting on self:** Set up a free Uptime Kuma instance to ping `/health` and email you if it's down

### 9.5 Backups

- Railway Postgres automated daily snapshots
- Weekly `pg_dump` to a local machine or S3 bucket via cron (manual setup)

---

## 10. Billing

### 10.1 MVP Pricing

| Tier | Price | Limits |
|------|-------|--------|
| **Trial** | Free, 14 days | 3 hosts, 24h retention, email alerts only |
| **Early Access** | $49/mo | 10 hosts, 72h retention, email + Slack alerts |

### 10.2 Stripe Integration

- Create a single Stripe Product ("NetWatch Early Access") with a $49/mo price
- On registration, create a Stripe Customer
- Trial: 14-day free trial on the subscription (Stripe handles trial logic)
- After trial: user must enter payment method or data stops being ingested
- Use Stripe Customer Portal for billing management (payment method, invoices, cancellation)
- Webhook endpoint: `/api/v1/webhooks/stripe` to handle:
  - `customer.subscription.updated` — update plan status
  - `customer.subscription.deleted` — downgrade to expired
  - `invoice.payment_failed` — mark account as past due

### 10.3 Enforcement

When an account is past-due or trial-expired:
- Agent ingest returns `402 Payment Required`
- Dashboard shows a banner: "Your trial has expired. Add a payment method to continue."
- Historical data is retained for 30 days after expiry, then deleted
- API keys continue to exist (so the agent doesn't need reconfiguration on reactivation)

---

## 11. Project Structure & Build

### 11.1 Repository Layout

```
netwatch/                          # existing repo
├── src/                           # existing TUI code (unchanged)
├── Cargo.toml                     # existing TUI binary
├── ...
│
├── crates/
│   └── netwatch-core/             # shared library crate (new)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── platform/
│           │   └── linux.rs       # extracted from src/platform/linux.rs
│           ├── collectors/
│           │   ├── health.rs      # ping logic (run_ping, parse_loss, parse_avg_rtt)
│           │   ├── config.rs      # gateway + DNS discovery
│           │   └── connections.rs # connection counting (Linux /proc/net)
│           └── types.rs           # Snapshot, InterfaceMetric, HealthMetric structs
│
├── netwatch-agent/                # agent daemon (new)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                # daemon entry point
│   │   ├── config.rs              # TOML config loading
│   │   ├── collector.rs           # orchestrates netwatch-core collectors
│   │   ├── sender.rs              # HTTPS POST with retry + buffering
│   │   └── host.rs                # host ID generation + metadata
│   └── netwatch-agent.service     # systemd unit file
│
├── netwatch-cloud/                # API server (new)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                # Axum server entry point
│   │   ├── routes/
│   │   │   ├── ingest.rs          # POST /api/v1/ingest
│   │   │   ├── auth.rs            # register, login, refresh
│   │   │   ├── hosts.rs           # host CRUD + metrics query
│   │   │   ├── alerts.rs          # alert rules CRUD + history
│   │   │   ├── account.rs         # account settings + API keys
│   │   │   └── webhooks.rs        # Stripe webhooks
│   │   ├── models/                # sqlx query structs
│   │   ├── auth.rs                # JWT + API key verification
│   │   ├── alerts/
│   │   │   ├── engine.rs          # background alert evaluation task
│   │   │   ├── state.rs           # alert state machine
│   │   │   └── notify.rs          # email + Slack notification senders
│   │   ├── db.rs                  # database pool setup
│   │   └── config.rs              # server config from env vars
│   ├── migrations/                # SQL migration files
│   └── Dockerfile
│
├── web/                           # Next.js frontend (new)
│   ├── package.json
│   ├── app/
│   │   ├── layout.tsx
│   │   ├── page.tsx               # host list (dashboard)
│   │   ├── login/page.tsx
│   │   ├── register/page.tsx
│   │   ├── hosts/[id]/page.tsx    # host detail
│   │   ├── alerts/page.tsx        # alert rules + history
│   │   └── settings/page.tsx      # account settings
│   ├── components/
│   │   ├── host-card.tsx
│   │   ├── metric-chart.tsx
│   │   ├── interface-table.tsx
│   │   ├── alert-rule-form.tsx
│   │   └── ...
│   └── lib/
│       ├── api.ts                 # API client
│       └── auth.ts                # JWT handling
│
└── install.sh                     # agent install script
```

### 11.2 Cargo Workspace

```toml
# Root Cargo.toml (add workspace)
[workspace]
members = [
    ".",                    # existing TUI
    "crates/netwatch-core",
    "netwatch-agent",
    "netwatch-cloud",
]
```

### 11.3 Key Dependencies

**netwatch-core:**
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
anyhow = "1"
nix = { version = "0.29", features = ["net", "hostname"] }
```

**netwatch-agent:**
```toml
[dependencies]
netwatch-core = { path = "../crates/netwatch-core" }
tokio = { version = "1", features = ["rt", "time", "net", "fs"] }
ureq = { version = "2", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
chrono = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**netwatch-cloud:**
```toml
[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
jsonwebtoken = "9"
argon2 = "0.5"
bcrypt = "0.17"
ureq = "2"   # for Slack + Resend API calls
tower-http = { version = "0.6", features = ["cors"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1"
```

---

## 12. What Is Explicitly Out of Scope

This list exists to prevent scope creep. None of these will be built for v1.

| Feature | Why Not |
|---------|---------|
| macOS / Windows agent | Linux only. Cross-platform is a hidden tax. |
| Packet capture | Requires root, complex, privacy nightmare. Not needed for 5 core metrics. |
| WebSocket real-time streaming | Polling every 15s is fine. Saves massive complexity. |
| GraphQL API | REST is enough. |
| SSO / OAuth | Email + password only for MVP. |
| Multi-user / teams | Single account per login. No roles, no invites. |
| Shared dashboards | One user sees their own hosts. |
| Self-hosted option | Cloud only. |
| AI insights | Distraction from core value. |
| Mobile app | Responsive web is enough. |
| Prometheus / Grafana export | Integration comes later, if customers ask. |
| Custom themes | Dark mode only. |
| PCAP storage / browser | Not a feature, it's an entire product. |
| Connection details (IPs, ports, processes) | Only connection count. Full details add privacy concerns and storage cost. |
| Redis / message queues | Postgres + in-memory is enough at MVP scale. |
| TimescaleDB | Plain Postgres with proper indexes handles 100 hosts fine. |
| Protobuf / MessagePack | JSON over HTTPS. |
| Agent auto-update | Manual update via install script. |
| Terraform provider | Nobody needs this at 10 customers. |
| Helm chart | No self-hosted, no Helm. |

---

## Appendix A: Implementation Order

A strict build sequence, designed to produce a working system as early as possible.

### Week 1–2: Core Infrastructure ✅ COMPLETE
1. ✅ Create `crates/netwatch-core` — shared types, platform collectors, health, system metrics
2. ✅ Create `netwatch-agent` — daemon collecting 9 metric types via HTTPS POST
3. ✅ Create `netwatch-cloud` — Axum server with all endpoints
4. ✅ Database schema + migrations (initial + system metrics)
5. ✅ Ingest endpoint (host upsert, snapshots, interfaces, system metrics)
6. ✅ Agent → API: HTTPS POST with API key auth + buffering

### Week 3–4: Web Dashboard ✅ COMPLETE
7. ✅ Auth endpoints (register, login, JWT)
8. ✅ Host list + host detail API endpoints (with CPU, memory, load avg)
9. ✅ Metrics query endpoint with time range filtering
10. ✅ Next.js 16 app: login, register, host list, host detail with 6 chart types
11. ✅ API key management (create, list, revoke)

### Week 5–6: Alerting ✅ COMPLETE
12. ✅ Alert rules CRUD (API + frontend)
13. ✅ Alert engine background task (30s evaluation loop)
14. ✅ Alert state machine (OK → Pending → Firing → Resolved)
15. ✅ Email notifications via Resend
16. ✅ Slack notifications via webhook
17. ✅ Default alert rules created on signup (4 rules)
18. ✅ Alert history page with filtering

### Week 7–8: Polish & Launch — IN PROGRESS
19. ✅ Install script (`curl | sh` with `--update` and `--remove`)
20. ✅ Agent self-update (`netwatch-agent update`)
21. ✅ Agent CLI (status, config, version, help)
22. ✅ Docker image + `agent.sh` manager script
23. ✅ Install script served from API (`/install.sh`)
24. ✅ CI/CD (GitHub Actions: check/test + cross-compiled releases)
25. ✅ Deploy to Railway (API + Web + Postgres)
26. ✅ Host offline detection (alert engine, 5 min timeout)
27. ❌ Stripe integration (trial + subscription)
28. ✅ Data retention cleanup job
29. ✅ Rate limiting
30. ✅ Landing page
31. ❌ Test with 3–5 design partners

**Next up:** Stripe billing, data retention, rate limiting, landing page.
