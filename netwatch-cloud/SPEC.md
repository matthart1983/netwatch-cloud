# NetWatch Cloud - System Specification

**Version:** 0.1.0  
**Last Updated:** March 31, 2026  
**Status:** Active Development

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Core Components](#core-components)
4. [API Specification](#api-specification)
5. [Data Model](#data-model)
6. [Authentication & Authorization](#authentication--authorization)
7. [Background Services](#background-services)
8. [Rate Limiting](#rate-limiting)
9. [Billing & Tenancy](#billing--tenancy)
10. [Alerting System](#alerting-system)
11. [Data Retention Policy](#data-retention-policy)
12. [Deployment](#deployment)
13. [Dependencies](#dependencies)

---

## Overview

**NetWatch Cloud** is a distributed network and system monitoring platform that aggregates metrics from remote agents, evaluates alert rules, and provides a multi-tenant SaaS interface for managing monitored hosts and receiving notifications.

### Key Features

- **Multi-tenant architecture** with isolated data per account
- **Agent-based monitoring** using API key authentication
- **Real-time metrics ingestion** from remote agents
- **Customizable alert rules** with multiple trigger types
- **Billing integration** with Stripe for subscription management
- **Multi-channel notifications** (Email via Resend, Slack webhooks)
- **Host management** with hardware and OS tracking
- **Interface and disk metrics** tracking
- **Automatic data retention** with hourly cleanup jobs

### Technology Stack

- **Language:** Rust
- **Web Framework:** Axum 0.8
- **Database:** PostgreSQL with SQLx
- **Runtime:** Tokio (async)
- **Authentication:** JWT for web users, API keys for agents
- **Billing:** Stripe webhooks
- **Notifications:** Resend (email), Slack webhooks

---

## Architecture

### High-Level System Design

```
┌─────────────────────────────────────────────────────────────┐
│                      External Agents                         │
│           (Remote monitoring agents, netwatch-core)         │
└──────────────┬──────────────────────────────────────────────┘
               │ HTTPS
               │ /api/v1/ingest (API Key Auth)
               ↓
┌─────────────────────────────────────────────────────────────┐
│                    NetWatch Cloud API                        │
│              (Rust/Axum REST Server)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Authentication Layer (JWT + API Key)                 │  │
│  │  Rate Limiting Middleware                             │  │
│  │  CORS & Tracing Layers                                │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────┐ │
│  │ Account Routes   │  │ Host Routes      │  │ Alert    │ │
│  │ (Auth, Billing)  │  │ (Query metrics)  │  │ Routes   │ │
│  └──────────────────┘  └──────────────────┘  └──────────┘ │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Background Services                                  │  │
│  │  • Alert Engine (30s cycle)                          │  │
│  │  • Retention Cleanup (hourly)                        │  │
│  │  • Stripe Webhook Handler                            │  │
│  └──────────────────────────────────────────────────────┘  │
└───────────┬──────────────────────────┬────────────────────┘
            │                          │
     ┌──────↓─────┐            ┌──────↓──────┐
     │ PostgreSQL  │            │ Slack/Email │
     │ Database    │            │ Webhooks    │
     └─────────────┘            └─────────────┘
```

### Component Organization

```
src/
├── main.rs              # Server initialization, middleware setup
├── auth.rs              # JWT & API key authentication
├── config.rs            # Configuration from environment variables
├── rate_limit.rs        # Rate limiting middleware and logic
├── retention.rs         # Hourly data cleanup job
├── alerts/
│   ├── mod.rs
│   ├── engine.rs        # Alert evaluation engine (30s loop)
│   └── notify.rs        # Alert notification dispatch
├── models/              # (Currently empty, would contain domain models)
└── routes/
    ├── mod.rs           # Route handler registration
    ├── health.rs        # Health checks and version info
    ├── install.rs       # Installation script download
    ├── ingest.rs        # Metrics ingestion from agents
    ├── auth.rs          # User login/registration
    ├── api_keys.rs      # API key management
    ├── hosts.rs         # Host querying and metrics retrieval
    ├── alerts.rs        # Alert rule management and history
    └── billing.rs       # Account management and Stripe webhooks
```

---

## Core Components

### 1. Web Server (`main.rs`)

**Purpose:** Initialize HTTP server, set up middleware, spawn background services

**Key Responsibilities:**
- Load configuration from environment
- Establish PostgreSQL connection pool (max 10 connections)
- Run SQL migrations
- Register all API routes
- Spawn background service tasks (alert engine, retention cleanup)
- Apply middleware (rate limiting, CORS, tracing, authentication)

**Middleware Stack (bottom to top):**
1. Rate limiting (per-route based on auth key or IP)
2. CORS (permissive)
3. HTTP tracing
4. Authentication extractors (JWT or API key)

---

### 2. Authentication Module (`auth.rs`)

**Purpose:** Handle user and agent authentication

#### User Authentication (JWT)

**Flow:**
1. User registers/logs in via `/api/v1/auth/register` or `/api/v1/auth/login`
2. Server returns JWT token
3. Client includes `Authorization: Bearer <token>` in requests
4. Token is validated and decoded to extract `account_id`

**Token Structure:**
- Claims: `{ sub: account_id, exp: unix_timestamp }`
- Expiration: 30 minutes
- Signing: HS256 with `JWT_SECRET`

**Extractor:** `AuthUser` - Axum extractor for authenticated web endpoints

#### Agent Authentication (API Keys)

**Flow:**
1. User creates API key via `/api/v1/account/api-keys` (POST)
2. Server generates key: `nw_ak_<8-random-chars><hash>`
3. Server stores: `key_prefix` (first 14 chars) and bcrypt `key_hash`
4. Agent includes `Authorization: Bearer <full_key>` in ingest requests
5. Server looks up by prefix, verifies with bcrypt, updates `last_used_at`

**Key Format:** `nw_ak_<8-random-chars><16-char-hash>`

**Extractor:** `AgentAuth` - Axum extractor for agent endpoints, returns `(account_id, api_key_id)`

---

### 3. Configuration (`config.rs`)

**Environment Variables:**

| Variable | Required | Default | Purpose |
|----------|----------|---------|---------|
| `DATABASE_URL` | ✓ | - | PostgreSQL connection string |
| `JWT_SECRET` | ✓ | - | Secret key for JWT signing |
| `BIND_ADDR` | ✗ | `0.0.0.0:3001` | Server bind address (override with `PORT` on Railway) |
| `RESEND_API_KEY` | ✗ | - | Resend API key for email notifications |
| `STRIPE_SECRET_KEY` | ✗ | - | Stripe API key for billing |
| `STRIPE_WEBHOOK_SECRET` | ✗ | - | Stripe webhook signing secret |

---

### 4. Rate Limiting (`rate_limit.rs`)

**Purpose:** Protect API from abuse with per-endpoint rate limits

**Implementation:**
- In-memory sliding window counter (HashMap of VecDeque)
- 10-minute background sweep task removes old entries
- 1-hour retention window for timestamps

**Rate Limit Rules:**

| Endpoint | Key | Limit | Window |
|----------|-----|-------|--------|
| `/api/v1/ingest` | Auth header | 10 requests | 60 seconds |
| `/api/v1/auth/login` | Client IP | 5 requests | 60 seconds |
| `/api/v1/auth/register` | Client IP | 3 requests | 3600 seconds |
| Other `/api/v1/*` | Auth header | 60 requests | 60 seconds |

**Key Extraction:**
- **Auth header:** Full authorization header value
- **IP:** `X-Forwarded-For` header (first IP) or connection socket address

---

## API Specification

### Base URL
```
https://api.netwatch.cloud
```

### Response Format
- **Content-Type:** `application/json`
- **Errors:** Standard HTTP status codes with JSON error messages (where applicable)

---

### Health & Installation

#### GET `/health`
Check server health.

**Response:**
```json
{
  "status": "ok"
}
```

#### GET `/version`
Get server version.

**Response:**
```json
{
  "version": "0.1.0"
}
```

#### GET `/install.sh`
Download shell script for agent installation.

**Response:** `text/plain` shell script

---

### Authentication Routes

#### POST `/api/v1/auth/register`
Create new account and get initial token pair.

**Rate Limit:** 3 per hour per IP

**Request:**
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response (201):**
```json
{
  "account_id": "uuid",
  "api_key": "nw_ak_...",
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc..."
}
```

**Errors:**
- `400 Bad Request` - Invalid input
- `409 Conflict` - Email already registered

#### POST `/api/v1/auth/login`
Authenticate user and get token pair.

**Rate Limit:** 5 per minute per IP

**Request:**
```json
{
  "email": "user@example.com",
  "password": "secure_password"
}
```

**Response (200):**
```json
{
  "account_id": "uuid",
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc..."
}
```

**Errors:**
- `401 Unauthorized` - Invalid credentials

#### POST `/api/v1/auth/refresh`
Obtain new access token without re-authenticating.

**Rate Limit:** 10 per minute per IP

**Request:**
```json
{
  "refresh_token": "eyJhbGc..."
}
```

**Response (200):**
```json
{
  "access_token": "eyJhbGc...",
  "refresh_token": "eyJhbGc..."
}
```

**Errors:**
- `400 Bad Request` - Missing refresh_token
- `401 Unauthorized` - Invalid or expired refresh token
- `500 Internal Server Error` - Account not found

**Notes:**
- Returns a new refresh token with each call (token rotation)
- Previous refresh token is implicitly invalidated
- Access token expiration: 15 minutes
- Refresh token expiration: 7 days

---

### Host Management Routes

#### GET `/api/v1/hosts`
List all hosts for authenticated user.

**Auth:** JWT

**Query Parameters:**
- None

**Response (200):**
```json
[
  {
    "id": "uuid",
    "hostname": "server-01",
    "os": "Ubuntu 22.04",
    "kernel": "5.15.0",
    "agent_version": "0.1.0",
    "cpu_model": "Intel Core i7-9700K",
    "cpu_cores": 8,
    "memory_total_bytes": 17179869184,
    "uptime_secs": 2592000,
    "is_online": true,
    "first_seen_at": "2026-03-20T10:00:00Z",
    "last_seen_at": "2026-03-31T14:22:00Z"
  }
]
```

#### GET `/api/v1/hosts/{id}`
Get detailed host information.

**Auth:** JWT

**Response (200):**
```json
{
  "id": "uuid",
  "hostname": "server-01",
  "os": "Ubuntu 22.04",
  "kernel": "5.15.0",
  "agent_version": "0.1.0",
  "cpu_model": "Intel Core i7-9700K",
  "cpu_cores": 8,
  "memory_total_bytes": 17179869184,
  "uptime_secs": 2592000,
  "is_online": true,
  "first_seen_at": "2026-03-20T10:00:00Z",
  "last_seen_at": "2026-03-31T14:22:00Z"
}
```

#### GET `/api/v1/hosts/{id}/metrics`
Get recent metrics snapshot for host.

**Auth:** JWT

**Query Parameters:**
- `limit` (optional): Number of recent snapshots to return (default: 1)

**Response (200):**
```json
[
  {
    "time": "2026-03-31T14:20:00Z",
    "connection_count": 42,
    "gateway_rtt_ms": 5.2,
    "gateway_loss_pct": 0.0,
    "dns_rtt_ms": 3.1,
    "dns_loss_pct": 0.0,
    "cpu_usage_pct": 25.5,
    "memory_used_bytes": 8589934592,
    "memory_available_bytes": 8589934592,
    "load_avg_1m": 1.2,
    "load_avg_5m": 1.4,
    "load_avg_15m": 1.3,
    "swap_used_bytes": 0,
    "disk_read_bytes": 1099511627776,
    "disk_write_bytes": 549755813888,
    "tcp_time_wait": 12,
    "tcp_close_wait": 2
  }
]
```

#### GET `/api/v1/hosts/{id}/disks`
Get disk metrics for host.

**Auth:** JWT

**Query Parameters:**
- `limit` (optional): Number of recent snapshots to return (default: 1)

**Response (200):**
```json
[
  {
    "time": "2026-03-31T14:20:00Z",
    "disks": [
      {
        "mount_point": "/",
        "device": "/dev/sda1",
        "total_bytes": 536870912000,
        "used_bytes": 268435456000,
        "available_bytes": 268435456000,
        "usage_pct": 50.0
      }
    ]
  }
]
```

#### GET `/api/v1/hosts/{id}/interfaces`
Get network interface metrics for host.

**Auth:** JWT

**Query Parameters:**
- `limit` (optional): Number of recent snapshots to return (default: 1)

**Response (200):**
```json
[
  {
    "time": "2026-03-31T14:20:00Z",
    "interfaces": [
      {
        "name": "eth0",
        "is_up": true,
        "rx_bytes_total": 1099511627776,
        "tx_bytes_total": 549755813888,
        "rx_bytes_delta": 1024000,
        "tx_bytes_delta": 512000,
        "rx_packets": 10000000,
        "tx_packets": 8000000,
        "rx_errors": 0,
        "tx_errors": 0,
        "rx_drops": 0,
        "tx_drops": 0
      }
    ]
  }
]
```

---

### Agent Ingest Route

#### POST `/api/v1/ingest`
Ingest metrics from monitoring agent.

**Auth:** API Key (Bearer token)

**Rate Limit:** 10 per minute per API key

**Request:**
```json
{
  "host": {
    "host_id": "uuid",
    "hostname": "server-01",
    "os": "Ubuntu 22.04",
    "kernel": "5.15.0",
    "cpu_model": "Intel Core i7-9700K",
    "cpu_cores": 8,
    "memory_total_bytes": 17179869184,
    "uptime_secs": 2592000
  },
  "agent_version": "0.1.0",
  "snapshots": [
    {
      "timestamp": "2026-03-31T14:20:00Z",
      "connection_count": 42,
      "health": {
        "gateway_ip": "192.168.1.1",
        "gateway_rtt_ms": 5.2,
        "gateway_loss_pct": 0.0,
        "dns_ip": "8.8.8.8",
        "dns_rtt_ms": 3.1,
        "dns_loss_pct": 0.0
      },
      "system": {
        "cpu_usage_pct": 25.5,
        "memory_total_bytes": 17179869184,
        "memory_used_bytes": 8589934592,
        "memory_available_bytes": 8589934592,
        "load_avg_1m": 1.2,
        "load_avg_5m": 1.4,
        "load_avg_15m": 1.3,
        "swap_total_bytes": 0,
        "swap_used_bytes": 0,
        "cpu_per_core": [25.5, 26.1, 24.8, 25.2, ...]
      },
      "disk_io": {
        "read_bytes": 1099511627776,
        "write_bytes": 549755813888
      },
      "disk_usage": [
        {
          "mount_point": "/",
          "device": "/dev/sda1",
          "total_bytes": 536870912000,
          "used_bytes": 268435456000,
          "available_bytes": 268435456000,
          "usage_pct": 50.0
        }
      ],
      "interfaces": [
        {
          "name": "eth0",
          "is_up": true,
          "rx_bytes": 1099511627776,
          "tx_bytes": 549755813888,
          "rx_bytes_delta": 1024000,
          "tx_bytes_delta": 512000,
          "rx_packets": 10000000,
          "tx_packets": 8000000,
          "rx_errors": 0,
          "tx_errors": 0,
          "rx_drops": 0,
          "tx_drops": 0
        }
      ],
      "tcp_time_wait": 12,
      "tcp_close_wait": 2
    }
  ]
}
```

**Constraints:**
- At least 1 snapshot required
- Maximum 100 snapshots per request
- Billing plan enforced: must be active or within trial period

**Response (200) - All snapshots accepted:**
```json
{
  "accepted": 95,
  "rejected": 0,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 200, "message": "OK" },
    { "index": 1, "status": 200, "message": "OK" },
    ...
  ]
}
```

**Response (207) - Partial success (some snapshots rejected):**
```json
{
  "accepted": 95,
  "rejected": 5,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 200, "message": "OK" },
    { "index": 5, "status": 400, "message": "Failed to insert snapshot" },
    { "index": 10, "status": 400, "message": "Failed to insert interface metrics" },
    { "index": 15, "status": 400, "message": "Failed to insert disk metrics" },
    ...
  ]
}
```

**Response (400) - All snapshots rejected:**
```json
{
  "accepted": 0,
  "rejected": 100,
  "host_id": "uuid",
  "results": [
    { "index": 0, "status": 400, "message": "Failed to insert snapshot" },
    ...
  ]
}
```

**Status Codes:**
- `200 OK` - All snapshots accepted
- `207 Multi-Status` - Partial success (some snapshots rejected, some accepted)
- `400 Bad Request` - All snapshots rejected OR invalid request format or empty snapshots
- `413 Payload Too Large` - More than 100 snapshots
- `401 Unauthorized` - Invalid API key
- `402 Payment Required` - Account plan expired or host limit exceeded

---

### API Key Management Routes

#### GET `/api/v1/account/api-keys`
List all API keys for account.

**Auth:** JWT

**Response (200):**
```json
[
  {
    "id": "uuid",
    "label": "Production Agent",
    "key_prefix": "nw_ak_12345678",
    "created_at": "2026-03-20T10:00:00Z",
    "last_used_at": "2026-03-31T14:22:00Z"
  }
]
```

#### POST `/api/v1/account/api-keys`
Create new API key.

**Auth:** JWT

**Request:**
```json
{
  "label": "Production Agent"
}
```

**Response (201):**
```json
{
  "id": "uuid",
  "key": "nw_ak_12345678abcdefghijklmnop",
  "label": "Production Agent",
  "key_prefix": "nw_ak_12345678",
  "created_at": "2026-03-31T14:20:00Z"
}
```

**Note:** The full `key` is only returned on creation and cannot be retrieved later.

#### DELETE `/api/v1/account/api-keys/{id}`
Revoke an API key.

**Auth:** JWT

**Response (204):** No content

**Errors:**
- `404 Not Found` - Key doesn't exist or not owned by user

---

### Alert Rules Routes

#### GET `/api/v1/alerts/rules`
List all alert rules for account.

**Auth:** JWT

**Query Parameters:**
- None

**Response (200):**
```json
[
  {
    "id": "uuid",
    "name": "High CPU Usage",
    "metric": "cpu_usage_pct",
    "condition": ">",
    "threshold": 80.0,
    "threshold_str": null,
    "duration_secs": 60,
    "severity": "warning",
    "host_id": "uuid or null",
    "enabled": true,
    "created_at": "2026-03-20T10:00:00Z"
  }
]
```

#### POST `/api/v1/alerts/rules`
Create new alert rule.

**Auth:** JWT

**Request:**
```json
{
  "name": "High CPU Usage",
  "metric": "cpu_usage_pct",
  "condition": ">",
  "threshold": 80.0,
  "host_id": "uuid or null (null = all hosts)",
  "duration_secs": 60,
  "severity": "warning"
}
```

**Supported Metrics:**
- `host_status` - Host online/offline (condition: `changes_to`, threshold_str: `offline`)
- `interface_status` - Interface up/down (condition: `changes_to`, threshold_str: `down`)
- `disk_usage_pct` - Disk usage percentage (condition: `>`, `<`, `==`)
- `cpu_usage_pct` - CPU usage percentage (condition: `>`, `<`, `==`)
- `gateway_rtt_ms` - Gateway latency (condition: `>`, `<`, `==`)
- `gateway_loss_pct` - Gateway packet loss (condition: `>`, `<`, `==`)
- `dns_rtt_ms` - DNS latency (condition: `>`, `<`, `==`)
- `dns_loss_pct` - DNS packet loss (condition: `>`, `<`, `==`)
- `memory_available_bytes` - Available memory (condition: `>`, `<`, `==`)
- `load_avg_1m` - 1-minute load average (condition: `>`, `<`, `==`)
- `connection_count` - Active connections (condition: `>`, `<`, `==`)
- `swap_used_bytes` - Swap memory used (condition: `>`, `<`, `==`)
- `disk_read_bytes` - Disk read bytes (condition: `>`, `<`, `==`)
- `disk_write_bytes` - Disk write bytes (condition: `>`, `<`, `==`)
- `tcp_time_wait` - TCP TIME_WAIT count (condition: `>`, `<`, `==`)
- `tcp_close_wait` - TCP CLOSE_WAIT count (condition: `>`, `<`, `==`)

**Response (201):**
```json
{
  "id": "uuid",
  "name": "High CPU Usage",
  "metric": "cpu_usage_pct",
  "condition": ">",
  "threshold": 80.0,
  "host_id": null,
  "duration_secs": 60,
  "severity": "warning",
  "enabled": true,
  "created_at": "2026-03-31T14:20:00Z"
}
```

#### PUT `/api/v1/alerts/rules/{id}`
Update alert rule.

**Auth:** JWT

**Request:** Same structure as POST (all fields optional)

**Response (200):** Updated rule object

#### DELETE `/api/v1/alerts/rules/{id}`
Delete alert rule.

**Auth:** JWT

**Response (204):** No content

#### GET `/api/v1/alerts/history`
Get alert event history.

**Auth:** JWT

**Query Parameters:**
- `limit` (optional): Max results (default: 100)
- `offset` (optional): Pagination offset (default: 0)
- `rule_id` (optional): Filter by rule ID
- `host_id` (optional): Filter by host ID

**Response (200):**
```json
[
  {
    "id": 12345,
    "rule_id": "uuid",
    "host_id": "uuid",
    "state": "firing",
    "metric_value": 85.2,
    "message": "WARNING: High CPU Usage on host server-01",
    "notified": true,
    "created_at": "2026-03-31T14:20:00Z"
  }
]
```

---

### Account & Billing Routes

#### GET `/api/v1/account`
Get account information.

**Auth:** JWT

**Response (200):**
```json
{
  "email": "user@example.com",
  "created_at": "2026-03-20T10:00:00Z",
  "plan": "early_access",
  "trial_ends_at": null,
  "stripe_customer_id": "cus_...",
  "notify_email": true,
  "slack_webhook": "https://hooks.slack.com/...",
  "portal_url": "https://billing.stripe.com/b/..."
}
```

#### PUT `/api/v1/account`
Update account preferences.

**Auth:** JWT

**Request:**
```json
{
  "notify_email": true,
  "slack_webhook": "https://hooks.slack.com/services/..."
}
```

**Response (204):** No content

#### GET `/api/v1/account/billing`
Get billing information.

**Auth:** JWT

**Response (200):**
```json
{
  "plan": "early_access",
  "trial_ends_at": null,
  "stripe_customer_id": "cus_...",
  "portal_url": "https://billing.stripe.com/b/..."
}
```

#### POST `/api/v1/webhooks/stripe`
Stripe webhook endpoint.

**Auth:** None (signature verification via HMAC)

**Headers:**
- `stripe-signature`: Signature header from Stripe

**Handles Events:**
- `customer.subscription.updated` → Updates plan to `early_access`, `past_due`, or `expired`
- `customer.subscription.deleted` → Sets plan to `expired`
- `invoice.payment_failed` → Sets plan to `past_due`

**Response (200):** Always returns OK (errors are logged)

**Note:** Cryptographic signature verification is pending (currently validates header format only). HMAC + SHA2 crates need to be added.

---

## Data Model

### Database Schema

#### `accounts` Table
```sql
id              UUID PRIMARY KEY
email           TEXT UNIQUE NOT NULL
password_hash   TEXT NOT NULL
created_at      TIMESTAMPTZ DEFAULT now()
notify_email    BOOLEAN DEFAULT true
slack_webhook   TEXT
stripe_customer_id TEXT
stripe_subscription_id TEXT
plan            TEXT DEFAULT 'trial' 
trial_ends_at   TIMESTAMPTZ
```

**Plans:** `trial`, `early_access`, `past_due`, `expired`

#### `api_keys` Table
```sql
id              UUID PRIMARY KEY
account_id      UUID REFERENCES accounts
key_hash        TEXT NOT NULL (bcrypt hash)
key_prefix      TEXT NOT NULL (indexed)
label           TEXT
created_at      TIMESTAMPTZ DEFAULT now()
last_used_at    TIMESTAMPTZ
```

#### `hosts` Table
```sql
id              UUID PRIMARY KEY
account_id      UUID REFERENCES accounts
api_key_id      UUID REFERENCES api_keys
hostname        TEXT NOT NULL
os              TEXT
kernel          TEXT
agent_version   TEXT
cpu_model       TEXT
cpu_cores       INTEGER
memory_total_bytes BIGINT
uptime_secs     BIGINT
first_seen_at   TIMESTAMPTZ DEFAULT now()
last_seen_at    TIMESTAMPTZ DEFAULT now()
is_online       BOOLEAN DEFAULT true
```

#### `snapshots` Table
```sql
id              BIGSERIAL PRIMARY KEY
host_id         UUID REFERENCES hosts
time            TIMESTAMPTZ NOT NULL
connection_count INTEGER
gateway_ip      TEXT
gateway_rtt_ms  DOUBLE PRECISION
gateway_loss_pct DOUBLE PRECISION
dns_ip          TEXT
dns_rtt_ms      DOUBLE PRECISION
dns_loss_pct    DOUBLE PRECISION
cpu_usage_pct   DOUBLE PRECISION
memory_total_bytes BIGINT
memory_used_bytes BIGINT
memory_available_bytes BIGINT
load_avg_1m     DOUBLE PRECISION
load_avg_5m     DOUBLE PRECISION
load_avg_15m    DOUBLE PRECISION
swap_total_bytes BIGINT
swap_used_bytes BIGINT
disk_read_bytes BIGINT
disk_write_bytes BIGINT
tcp_time_wait   INTEGER
tcp_close_wait  INTEGER
cpu_per_core    TEXT (JSON array as text)
```

**Index:** `(host_id, time DESC)`

#### `interface_metrics` Table
```sql
id              BIGSERIAL PRIMARY KEY
snapshot_id     BIGINT REFERENCES snapshots ON DELETE CASCADE
host_id         UUID
time            TIMESTAMPTZ NOT NULL
name            TEXT NOT NULL
is_up           BOOLEAN NOT NULL
rx_bytes_total  BIGINT
tx_bytes_total  BIGINT
rx_bytes_delta  BIGINT
tx_bytes_delta  BIGINT
rx_packets      BIGINT
tx_packets      BIGINT
rx_errors       BIGINT
tx_errors       BIGINT
rx_drops        BIGINT
tx_drops        BIGINT
```

**Indexes:** `(host_id, time DESC)`, `(snapshot_id)`

#### `disk_metrics` Table
```sql
id              BIGSERIAL PRIMARY KEY
snapshot_id     BIGINT REFERENCES snapshots ON DELETE CASCADE
host_id         UUID
time            TIMESTAMPTZ NOT NULL
mount_point     TEXT NOT NULL
device          TEXT NOT NULL
total_bytes     BIGINT
used_bytes      BIGINT
available_bytes BIGINT
usage_pct       DOUBLE PRECISION
```

**Index:** `(host_id, time DESC)`

#### `alert_rules` Table
```sql
id              UUID PRIMARY KEY
account_id      UUID REFERENCES accounts
host_id         UUID REFERENCES hosts (nullable, null = all hosts)
name            TEXT NOT NULL
metric          TEXT NOT NULL
condition       TEXT NOT NULL
threshold       DOUBLE PRECISION (nullable for string conditions)
threshold_str   TEXT (for non-numeric conditions)
duration_secs   INTEGER DEFAULT 60
severity        TEXT DEFAULT 'warning'
enabled         BOOLEAN DEFAULT true
created_at      TIMESTAMPTZ DEFAULT now()
```

**Index:** `(account_id)`

#### `alert_events` Table
```sql
id              BIGSERIAL PRIMARY KEY
rule_id         UUID REFERENCES alert_rules
host_id         UUID REFERENCES hosts
state           TEXT ('firing', 'resolved')
metric_value    DOUBLE PRECISION (nullable)
message         TEXT NOT NULL
notified        BOOLEAN DEFAULT false
created_at      TIMESTAMPTZ DEFAULT now()
```

**Indexes:** `(rule_id, created_at DESC)`, `(host_id, created_at DESC)`

---

## Authentication & Authorization

### Multi-Tenant Isolation

All queries are filtered by `account_id` to ensure complete data isolation between accounts. This is enforced at every route handler.

### JWT (Web Users)

#### Access Token

- **Issued by:** `/api/v1/auth/register`, `/api/v1/auth/login`, `/api/v1/auth/refresh`
- **Format:** HS256 signed JWT
- **Payload:** `{ sub: account_id, token_type: "access", exp: unix_timestamp }`
- **Expiration:** 15 minutes
- **Storage:** Client-side (typically in browser localStorage or sessionStorage)
- **Transmission:** `Authorization: Bearer <access_token>`
- **Usage:** All authenticated REST API calls

#### Refresh Token

- **Issued by:** `/api/v1/auth/register`, `/api/v1/auth/login`, `/api/v1/auth/refresh`
- **Format:** HS256 signed JWT
- **Payload:** `{ sub: account_id, token_type: "refresh", exp: unix_timestamp }`
- **Expiration:** 7 days
- **Storage:** Client-side (typically in secure httpOnly cookie or localStorage)
- **Transmission:** Request body (JSON) to `/api/v1/auth/refresh`
- **Usage:** Obtain new access/refresh token pair without re-authenticating

#### Token Rotation

- Each call to `/api/v1/auth/refresh` returns a **new refresh token** (old one implicitly invalidated)
- Prevents token replay attacks by issuing unique tokens per refresh cycle
- Client must update stored refresh token on each refresh response

### API Keys (Agents)

- **Format:** `nw_ak_<8-random><16-char-hash>`
- **Storage:** Server stores `key_prefix` (14 chars) and bcrypt hash
- **Validation:** Lookup by prefix, then bcrypt verification
- **Tracking:** `last_used_at` timestamp updated on each use
- **Transmission:** `Authorization: Bearer <full_key>`

### Password Security

- **Hashing:** bcrypt (for both account passwords and API key hashes)
- **Minimum strength:** None enforced at API level (client responsibility)

---

## Background Services

### Alert Engine (`alerts/engine.rs`)

**Cycle Frequency:** Every 30 seconds

**Algorithm:**
1. Mark hosts offline if no snapshot in 5 minutes
2. Load all enabled alert rules
3. For each rule, determine applicable hosts (specific or all)
4. For each host, evaluate condition:
   - **Ok → Pending:** Condition met, start timer
   - **Pending → Firing:** Duration threshold reached, send notification
   - **Firing → Ok:** Condition no longer met, send resolution notification
   - **Ok → Ok:** No change
5. Persist alert events to database
6. Dispatch notifications (email, Slack)

**State Tracking:**
- In-memory HashMap: `{ (rule_id, host_id) → AlertState }`
- States: `Ok`, `Pending { since }`, `Firing`, `Resolved`

**Metrics Evaluated:**
- Host status (online/offline)
- Interface status (up/down)
- Numeric thresholds (CPU, memory, disk, latency, loss, etc.)

---

### Data Retention Job (`retention.rs`)

**Cycle Frequency:** Every 60 minutes

**Operations:**
1. Delete snapshots older than 72 hours
   - Cascades to `interface_metrics` and `disk_metrics` via ON DELETE CASCADE
2. Delete alert events older than 30 days
3. Mark hosts offline if no snapshot in 5 minutes

**Rationale:** Balance storage cost with data retention for historical analysis

---

### Stripe Webhook Handler (`routes/billing.rs`)

**Endpoint:** `POST /api/v1/webhooks/stripe`

**Security:** HMAC-SHA256 signature verification enabled when `STRIPE_WEBHOOK_SECRET` is configured.

**Signature Verification Details:**

1. **Header Format:** `t=<timestamp>,v1=<signature>[,v1=<signature>...]`
2. **Algorithm:** HMAC-SHA256 with webhook secret as key
3. **Signed Content:** `"<timestamp>.<raw_request_body>"`
4. **Timestamp Validation:** Must be within 5 minutes (prevents replay attacks)
5. **Comparison:** Constant-time comparison to prevent timing attacks

**Verification Process:**
- Parse `stripe-signature` header for timestamp (`t=`) and signature(s) (`v1=`)
- Validate timestamp is within 5-minute window
- For each v1 signature:
  - Decode hex-encoded signature
  - Compute HMAC-SHA256(`secret`, `timestamp.payload`)
  - Use constant-time comparison to verify
- Reject if no valid signature found
- Log all verification failures with context

**Handled Events:**

| Event | Action |
|-------|--------|
| `customer.subscription.updated` | Update plan based on subscription status |
| `customer.subscription.deleted` | Set plan to `expired` |
| `invoice.payment_failed` | Set plan to `past_due` |

**Status → Plan Mapping:**

| Subscription Status | Plan |
|--------------------|----|
| `active`, `trialing` | `early_access` |
| `past_due` | `past_due` |
| `canceled`, `unpaid`, `incomplete_expired` | `expired` |

---

## Rate Limiting

### Strategy

**Sliding window counter** using in-memory HashMap with automatic cleanup.

### Configuration

| Endpoint | Auth Method | Limit | Window | Key |
|----------|-------------|-------|--------|-----|
| `/api/v1/ingest` | API Key | 10 req | 60 sec | Auth header |
| `/api/v1/auth/login` | None | 5 req | 60 sec | Client IP |
| `/api/v1/auth/register` | None | 3 req | 3600 sec | Client IP |
| Other `/api/v1/*` | JWT | 60 req | 60 sec | Auth header |
| Non-API routes | - | Unlimited | - | - |

### Cleanup

- **Sweep Frequency:** Every 10 minutes
- **Retention Window:** 1 hour
- **Lock-free Design:** Single Mutex over HashMap (acceptable for small user base)

---

## Billing & Tenancy

### Plan Types

| Plan | Host Limit | Ingest Limit | Features |
|------|-----------|-------------|----------|
| `trial` | 3 | Unlimited* | All features |
| `early_access` | 10 | Unlimited | All features |
| `past_due` | 0 | Blocked | Account under payment review |
| `expired` | 0 | Blocked | Account expired or subscription canceled |

*Trial enforced via `trial_ends_at` timestamp

### Billing Flow

1. **New Account:** Created with `plan: trial` and 14-day `trial_ends_at` (not enforced in code yet)
2. **During Trial:** User can test with up to 3 hosts
3. **Upgrade to Paid:**
   - User visits `/api/v1/account/billing` to get Stripe portal URL
   - Portal URL created via Stripe API
   - Subscription created in Stripe
4. **Webhook Updates:** Stripe webhooks update plan status
5. **Usage Enforcement:** 
   - Ingest blocked if plan is `past_due` or `expired`
   - Host limit enforced unless creating snapshot for existing host

### Stripe Integration

**Webhook Secret:** Verified via HMAC (currently stub implementation)

**TODO:** Add `hmac` and `sha2` crates for proper cryptographic verification

---

## Alerting System

### Rule Evaluation

Each rule defines:
- **Metric:** What to measure
- **Condition:** How to evaluate (operators or string matching)
- **Threshold:** Numeric threshold (optional for string conditions)
- **Duration:** How long condition must persist before firing
- **Severity:** `info`, `warning`, `critical`
- **Host ID:** Specific host or NULL for all hosts

### State Machine

```
Ok ──condition_met──→ Pending {since: now}
             ↓
          duration_exceeded
             ↓
         Firing ──condition_not_met──→ Resolved
             ↓
          condition_not_met (send resolution notification)
             ↓
            Ok
```

### Notification Channels

**Email (via Resend):**
- Sent if `notify_email: true` on account
- Format: Plain text with alert message, hostname, severity

**Slack:**
- Sent to webhook URL if `slack_webhook` is set
- Format: Slack message with rich formatting

**Implementation:** `alerts/notify.rs` (details in source)

---

## Data Retention Policy

### Snapshot Data (Metrics)

**Retention:** 72 hours (3 days)

**Rationale:** Covers typical incident investigation window while minimizing storage costs

**Cascade:** Deletes automatically cascade to related `interface_metrics` and `disk_metrics`

### Alert Events

**Retention:** 30 days

**Rationale:** Provides month-long audit trail for compliance and incident review

### Hosts

**Retention:** Indefinite

**Offline Marking:** Host marked `is_online: false` if no snapshot in 5 minutes

---

## Deployment

### Docker

**Image:** Multi-stage build (builder + runtime)

```dockerfile
FROM rust:1-bookworm AS builder
# Build Rust binary

FROM debian:bookworm-slim
# Runtime with CA certificates
```

**Environment Variables:** Set at container startup

**Port:** Configurable via `BIND_ADDR` or `PORT` environment variable

### Database Migrations

**Tool:** SQLx compile-time verified migrations

**Location:** `migrations/` directory

**Execution:** Automatic on server startup via `sqlx::migrate!("./migrations")`

### Supported Platforms

- **Railway** (auto PORT env var detection)
- **Docker/Kubernetes** (BIND_ADDR override)
- **Local development** (default 0.0.0.0:3001)

---

## Dependencies

### Core Runtime

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1 | Async runtime |
| `axum` | 0.8 | Web framework |
| `sqlx` | 0.8 | Database driver |
| `serde` / `serde_json` | Latest | Serialization |

### Security

| Crate | Version | Purpose |
|-------|---------|---------|
| `jsonwebtoken` | 9 | JWT creation/verification |
| `bcrypt` | 0.17 | Password/key hashing |

### Database & IDs

| Crate | Version | Purpose |
|-------|---------|---------|
| `sqlx` | 0.8 | PostgreSQL + migrations |
| `uuid` | 1 | ID generation |
| `chrono` | 0.4 | Timestamps |

### Observability

| Crate | Version | Purpose |
|-------|---------|---------|
| `tracing` | 0.1 | Structured logging |
| `tracing-subscriber` | 0.3 | Log output |
| `tower-http` | 0.6 | HTTP middleware (CORS, tracing) |

### External APIs

| Crate | Version | Purpose |
|-------|---------|---------|
| `ureq` | 2 | Stripe API calls |

### Error Handling

| Crate | Version | Purpose |
|-------|---------|---------|
| `anyhow` | 1 | Error propagation |

### Local Crates

| Crate | Path | Purpose |
|-------|------|---------|
| `netwatch-core` | `../crates/netwatch-core` | Shared types (IngestRequest, etc.) |

---

## Future Considerations

### Performance Optimizations

- [ ] Implement connection pooling with dynamic sizing
- [ ] Add caching layer (Redis) for frequently accessed hosts/metrics
- [ ] Optimize alert evaluation with indexed metric lookups
- [ ] Batch insert optimizations for high-volume ingest

### Feature Enhancements

- [ ] Custom notification templates
- [ ] PagerDuty / Opsgenie integration
- [ ] Scheduled report generation
- [ ] Metrics aggregation and trend analysis
- [ ] Alert rule templates
- [ ] Multi-account organization support

### Security Hardening

- [ ] Complete Stripe webhook signature verification (add hmac/sha2 crates)
- [ ] Rate limiting via Redis for distributed deployments
- [ ] API key rotation mechanism
- [ ] Audit logging for sensitive operations
- [ ] IP allowlisting for API keys

### Operational Improvements

- [ ] Structured logging with request tracing
- [ ] Metrics export (Prometheus format)
- [ ] Health check enhancements
- [ ] Graceful shutdown handling
- [ ] Database connection health monitoring

---

**End of Specification Document**
