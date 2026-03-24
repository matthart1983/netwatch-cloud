# NetWatch — SaaS Business Plan & Technical Roadmap

## Executive Summary

NetWatch is currently a free, open-source TUI tool for real-time network diagnostics. This document outlines the plan to evolve it into **NetWatch Cloud** — a paid SaaS platform that extends the TUI agent with a centralized web dashboard, multi-host fleet monitoring, historical data retention, alerting, and team collaboration. The open-source TUI remains free and becomes the **agent** that feeds data upstream.

**Business model:** Open-core. The TUI stays MIT-licensed and gains users organically. Revenue comes from the hosted cloud platform that adds the features solo operators and teams will pay for — history, alerts, multi-host views, and collaboration.

---

## Part 1: Business Plan

### 1.1 Problem

Engineers and SREs currently have two options for network monitoring:

1. **Enterprise tools** (Datadog, New Relic, PRTG, SolarWinds) — expensive ($15–50+/host/month), complex to deploy, overkill for small/mid teams
2. **CLI tools** (iftop, nethogs, ss, tcpdump) — free but ephemeral, no history, no dashboards, no collaboration, no alerting

There is no lightweight, developer-friendly network monitoring tool that bridges the gap: easy to install, cheap, with a modern web UI and historical analysis. NetWatch is perfectly positioned to fill this gap.

### 1.2 Target Customers

| Segment | Size | Pain Point | Willingness to Pay |
|---------|------|------------|---------------------|
| **Indie developers & homelabbers** | Millions | Want visibility into their servers/VPS without enterprise complexity | $5–15/mo |
| **Small DevOps teams (2–20)** | Hundreds of thousands | Need multi-host monitoring, alerting, incident context without Datadog pricing | $20–100/mo |
| **Mid-market SRE teams (20–100)** | Tens of thousands | Need fleet-wide network observability, SSO, audit logs, compliance | $200–1000/mo |
| **MSPs & consultancies** | Thousands | Monitor client infrastructure, need multi-tenant dashboards | $500–2000/mo |

### 1.3 Competitive Landscape

| Competitor | Strengths | Weaknesses | NetWatch Advantage |
|------------|-----------|------------|---------------------|
| **Datadog NPM** | Deep integrations, enterprise trust | $5/host/mo minimum, complex, requires agent + config | 10x cheaper, installs in one command, open-source agent |
| **ntopng** | Powerful, open-source | Complex setup, dated UI, no cloud option | Modern UX, cloud-native, zero-config agent |
| **Wireshark** | Gold standard for packet analysis | Desktop only, no monitoring/alerting, no collaboration | Continuous monitoring + web UI + team features |
| **Tailscale/Nebula** | Network layer tools | Not monitoring tools | Complementary — NetWatch monitors what they connect |
| **Uptime Kuma** | Simple, self-hosted monitoring | HTTP/ping only, no deep network analysis | Full L2-L7 visibility, packet capture, topology |

### 1.4 Pricing Tiers

| Tier | Price | Hosts | Retention | Features |
|------|-------|-------|-----------|----------|
| **Free (TUI only)** | $0 | 1 (local) | None (real-time only) | Full TUI — all current features, forever free |
| **Starter** | $9/mo | Up to 3 hosts | 7 days | Web dashboard, basic alerts (email), 1 user |
| **Pro** | $29/mo | Up to 10 hosts | 30 days | All alerts (Slack, PagerDuty, webhook), team (3 users), PCAP storage (1 GB), API access |
| **Team** | $79/mo | Up to 50 hosts | 90 days | Unlimited users, SSO (Google/GitHub), shared dashboards, PCAP storage (10 GB), custom alert rules |
| **Enterprise** | Custom | Unlimited | 1 year | Self-hosted option, SAML/OIDC SSO, audit logs, SLA, dedicated support, on-prem deployment |

**Annual discount:** 20% off (2 months free).

### 1.5 Revenue Projections (Conservative)

| Month | Free Users | Paid Users | MRR | ARR |
|-------|-----------|------------|-----|-----|
| 6 | 2,000 | 20 | $580 | $6,960 |
| 12 | 8,000 | 100 | $2,900 | $34,800 |
| 18 | 20,000 | 350 | $10,150 | $121,800 |
| 24 | 50,000 | 1,000 | $29,000 | $348,000 |

Assumes 1–2% free-to-paid conversion rate, average $29/mo blended ARPU.

### 1.6 Go-to-Market Strategy

#### Phase 1: Community & Credibility (Months 1–6)
- Continue growing open-source TUI — target 5,000+ GitHub stars
- Write technical blog posts (Rust performance, packet capture internals, eBPF)
- Post on r/rust, r/selfhosted, r/homelab, r/netsec, r/sysadmin, Hacker News
- Produce YouTube demo videos and tutorials
- Engage in DevOps/SRE communities (Slack groups, Discord, forums)
- Offer free beta access to the cloud platform to early TUI users

#### Phase 2: Launch & Early Revenue (Months 6–12)
- Launch NetWatch Cloud with Starter and Pro tiers
- Product Hunt launch
- Conference talks (RustConf, KubeCon, local meetups)
- Dev-focused content marketing (blog, newsletter)
- Referral program: give a month, get a month

#### Phase 3: Scale (Months 12–24)
- Launch Team and Enterprise tiers
- Partner integrations (PagerDuty, Slack, Grafana, Prometheus)
- SOC 2 Type II certification for enterprise sales
- Hire first sales engineer for enterprise pipeline
- Explore MSP/white-label channel

### 1.7 Key Metrics to Track

| Metric | Target (Month 12) |
|--------|--------------------|
| GitHub stars | 5,000+ |
| Monthly TUI downloads (crates.io + brew + binary) | 10,000+ |
| Registered cloud users | 2,000+ |
| Paid subscribers | 100+ |
| MRR | $2,900+ |
| Churn rate | < 5%/month |
| Free-to-paid conversion | > 1.5% |
| NPS | > 50 |

### 1.8 Costs & Infrastructure

| Item | Monthly Cost (at launch) | Monthly Cost (at scale) |
|------|--------------------------|--------------------------|
| Cloud infrastructure (AWS/Fly.io) | $200 | $2,000 |
| Domain, DNS, SSL | $10 | $10 |
| Transactional email (Resend/Postmark) | $20 | $100 |
| Payment processing (Stripe) | 2.9% + $0.30/txn | 2.9% + $0.30/txn |
| Error tracking (Sentry) | $0 (free tier) | $26 |
| GeoIP database (MaxMind) | $0 (GeoLite2 free) | $100 (commercial) |
| Your time (opportunity cost) | Priceless | Hire first employee at ~$5K MRR |

**Break-even:** ~$300/mo fixed costs → ~11 Starter customers or ~4 Pro customers.

---

## Part 2: Technical Roadmap

### 2.1 Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                        NetWatch Cloud                            │
│                      (Web Dashboard)                             │
│              React/Next.js + WebSocket + REST API                │
├──────────────────────────────────────────────────────────────────┤
│                        API Gateway                               │
│              (Axum — Rust HTTP/WebSocket server)                  │
├──────────────┬────────────────┬───────────────┬─────────────────┤
│  Auth/Billing│  Ingest Engine │  Query Engine │  Alert Engine   │
│  (Stripe,    │  (time-series  │  (historical  │  (rule eval,    │
│   JWT, OIDC) │   ingestion)   │   queries)    │   notifications)│
├──────────────┴────────────────┴───────────────┴─────────────────┤
│                    Data Storage Layer                             │
│         TimescaleDB (metrics) + S3 (PCAPs) + Redis (sessions)    │
└──────────────────────────────────────────────────────────────────┘
          ▲              ▲              ▲
          │              │              │
     ┌────┴────┐    ┌────┴────┐    ┌────┴────┐
     │  Agent  │    │  Agent  │    │  Agent  │
     │  (TUI)  │    │  (TUI)  │    │  (TUI)  │
     │ Host A  │    │ Host B  │    │ Host C  │
     └─────────┘    └─────────┘    └─────────┘
```

### 2.2 Technical Phases

---

#### Phase 0: Foundation (Weeks 1–4)

**Goal:** Prepare the TUI codebase for agent mode and set up the backend skeleton.

##### TUI Changes

- [ ] **Agent mode flag:** Add `--agent` CLI flag that runs netwatch headless (no TUI rendering), collecting and forwarding data
- [ ] **Configuration for cloud endpoint:** Add `cloud.endpoint`, `cloud.api_key` to TOML config
- [ ] **Data serialization layer:** Define protobuf or MessagePack schemas for all collector outputs:
  - `InterfaceSnapshot` — interface stats, rates, status
  - `ConnectionSnapshot` — active connections with process info
  - `HealthSnapshot` — gateway/DNS RTT and loss
  - `PacketSummary` — aggregated protocol stats (not raw packets in default mode)
  - `TopologySnapshot` — current network topology
- [ ] **Heartbeat system:** Agent sends periodic heartbeat with host metadata (hostname, OS, version, uptime)
- [ ] **Refactor collectors into `netwatch-core` library crate:** Extract collector logic from the binary so the API server can reuse data structures

##### Backend Skeleton

- [ ] **API server:** Axum-based Rust server with:
  - `POST /api/v1/ingest` — receive agent snapshots (authenticated via API key)
  - `GET /api/v1/hosts` — list registered hosts
  - `GET /api/v1/hosts/:id/latest` — latest snapshot for a host
  - WebSocket endpoint for real-time streaming to web dashboard
- [ ] **Database setup:** TimescaleDB (PostgreSQL extension) for time-series metrics
  - Schema: `host_metrics` hypertable partitioned by time
  - Retention policies per tier
- [ ] **Authentication:** API key generation and validation for agents; JWT for web users
- [ ] **Docker Compose dev environment:** Postgres/TimescaleDB + Redis + API server + web frontend

##### Deliverables
- Agent can run headless and send data to a local API server
- API server receives and stores snapshots
- Basic database schema operational

---

#### Phase 1: Web Dashboard MVP (Weeks 5–12)

**Goal:** Ship the minimum viable cloud product — a web dashboard showing real-time and recent historical data from connected agents.

##### Backend

- [ ] **User registration & login:** Email/password with bcrypt, JWT tokens, email verification
- [ ] **Stripe integration:** Subscription management, webhook handlers for payment events
- [ ] **Host management API:**
  - `POST /api/v1/hosts/register` — register a new agent, returns API key
  - `DELETE /api/v1/hosts/:id` — deregister
  - `GET /api/v1/hosts/:id/metrics?from=&to=&resolution=` — historical queries with automatic downsampling
- [ ] **WebSocket hub:** Real-time forwarding of agent snapshots to connected web clients
- [ ] **Retention enforcement:** Background job that deletes data older than tier allows
- [ ] **Rate limiting:** Per-agent ingest rate limits to prevent abuse

##### Frontend (Next.js + Tailwind + shadcn/ui)

- [ ] **Dashboard page:** Fleet overview — all hosts with status indicators, key metrics
- [ ] **Host detail page:** Single-host deep dive mirroring TUI tabs:
  - Interfaces panel with bandwidth charts (Recharts/Tremor)
  - Connections table (sortable, filterable)
  - Health panel with latency graphs
  - Topology visualization (D3.js or vis.js)
  - Timeline (Gantt chart, similar to TUI timeline tab)
- [ ] **Settings page:** Account management, API keys, billing (Stripe Customer Portal)
- [ ] **Responsive layout:** Works on desktop and tablet

##### Agent Updates

- [ ] **Automatic reconnection:** Exponential backoff on connection failures
- [ ] **Local buffering:** Queue snapshots locally if cloud is unreachable, replay on reconnect
- [ ] **Bandwidth-conscious mode:** Configurable snapshot interval (default 10s, minimum 5s)
- [ ] **Version reporting:** Agent reports its version; server can suggest updates

##### Deliverables
- Working cloud product: sign up, install agent, see data in browser
- Stripe billing operational
- Launch Starter and Pro tiers

---

#### Phase 2: Alerting & Notifications (Weeks 13–18)

**Goal:** Add the feature that justifies ongoing payment — proactive alerting.

##### Alert Engine

- [ ] **Rule definition system:**
  ```
  Rules are stored as structured objects, not a DSL (initially):
  - Metric: bandwidth_rx, bandwidth_tx, packet_loss, latency_gateway,
            latency_dns, connection_count, interface_status, error_rate
  - Condition: >, <, ==, !=, changes_to
  - Threshold: numeric value or enum (e.g., "down")
  - Duration: how long condition must hold before firing (default 60s)
  - Severity: info, warning, critical
  ```
- [ ] **Evaluation loop:** Runs every 10s, checks all active rules against latest metrics per host
- [ ] **State machine per rule:** OK → PENDING → FIRING → RESOLVED (prevents flapping)
- [ ] **Notification channels:**
  - Email (via Resend/Postmark API)
  - Slack (incoming webhook)
  - PagerDuty (Events API v2)
  - Generic webhook (POST with JSON payload)
  - In-app notification bell
- [ ] **Alert history:** Persistent log of all alert state changes with timestamps
- [ ] **Mute/snooze:** Silence alerts per host or per rule for a duration

##### Frontend

- [ ] **Alert rules page:** Create/edit/delete alert rules with a form UI
- [ ] **Alert history page:** Timeline of alerts with filtering by host, severity, status
- [ ] **Notification settings:** Configure channels per user
- [ ] **Alert badges:** Visual indicators on host cards and nav bar

##### Built-in Default Rules (Starter-friendly)

- Interface goes DOWN
- Packet loss > 5%
- Gateway latency > 100ms
- DNS latency > 200ms
- Bandwidth spike > 2x average

##### Deliverables
- Users receive alerts via email/Slack/PagerDuty when network issues occur
- Default rules work out of the box with zero configuration

---

#### Phase 3: Packet Capture Cloud Features (Weeks 19–26)

**Goal:** Differentiate from generic monitoring — bring NetWatch's unique packet analysis capabilities to the cloud.

##### PCAP Upload & Storage

- [ ] **On-demand capture trigger:** Web UI can request an agent to start a capture with filters
  - `POST /api/v1/hosts/:id/capture` — start capture (BPF filter, duration, max packets)
  - `DELETE /api/v1/hosts/:id/capture` — stop capture
- [ ] **PCAP upload:** Agent streams PCAP data to S3-compatible storage (AWS S3 or MinIO)
- [ ] **PCAP browser:** Web UI lists stored captures with metadata (host, time, size, filter)
- [ ] **PCAP analysis view:** Browser-based packet viewer (inspired by CloudShark):
  - Packet list with protocol coloring
  - Protocol decode tree
  - Hex/ASCII dump
  - Display filters (reuse TUI filter syntax)
- [ ] **Storage quotas:** Enforced per tier (1 GB Pro, 10 GB Team)
- [ ] **Auto-expiry:** PCAPs auto-delete after retention period

##### Protocol Statistics API

- [ ] **Protocol distribution over time:** Historical protocol mix charts
- [ ] **Top talkers:** Ranked list of IPs/hosts by traffic volume over time
- [ ] **DNS query log:** Searchable log of all DNS queries with response codes

##### Deliverables
- Users can trigger remote packet captures and analyze them in the browser
- Unique capability — no competitor offers this at this price point

---

#### Phase 4: Team & Collaboration (Weeks 27–34)

**Goal:** Enable multi-user teams and shared workflows — the features that drive Team tier adoption.

##### Multi-User & Permissions

- [ ] **Organizations:** Users belong to an org; org owns hosts and billing
- [ ] **Roles:** Owner, Admin, Member, Read-only
- [ ] **Invite flow:** Email invitations with role assignment
- [ ] **SSO:** Google and GitHub OAuth2 login
- [ ] **Audit log:** Track who did what (captures, alert changes, config changes)

##### Shared Dashboards

- [ ] **Custom dashboards:** Drag-and-drop widgets (bandwidth chart, connection table, health panel)
- [ ] **Dashboard sharing:** Share a dashboard within the org or via public link
- [ ] **Annotations:** Add notes to time ranges ("deployed v2.1", "DDoS incident")

##### Incident Workflow

- [ ] **Incidents:** Create an incident from an alert, attach hosts and captures
- [ ] **Timeline view:** Unified timeline showing alerts, metric changes, captures, and annotations
- [ ] **Postmortem template:** Markdown-based template with auto-populated data

##### Deliverables
- Teams can collaborate on network issues
- Shared dashboards provide common operational views
- Incident workflow reduces mean-time-to-resolution

---

#### Phase 5: Enterprise & Scale (Weeks 35–52)

**Goal:** Enterprise readiness and advanced features for large deployments.

##### Enterprise Auth

- [ ] **SAML 2.0 / OIDC SSO:** Okta, Azure AD, OneLogin integration
- [ ] **SCIM provisioning:** Automatic user provisioning/deprovisioning
- [ ] **MFA enforcement:** Require 2FA for all org members

##### Self-Hosted Option

- [ ] **Helm chart:** Deploy NetWatch Cloud on customer's Kubernetes cluster
- [ ] **Docker Compose production stack:** Single-node deployment option
- [ ] **Air-gapped support:** No external dependencies required
- [ ] **Upgrade path:** Automated migrations between versions

##### Advanced Features

- [ ] **AI insights (cloud-powered):** Replace local Ollama with cloud LLM analysis
  - Automatic anomaly detection across all hosts
  - Natural language querying ("show me all DNS failures in the last hour")
  - Weekly network health digest email
- [ ] **Prometheus/Grafana export:** `/metrics` endpoint in Prometheus format
- [ ] **API v2:** Full REST + GraphQL API for custom integrations
- [ ] **Terraform provider:** Manage hosts, alerts, and dashboards as code
- [ ] **Agent auto-update:** Secure automatic agent updates via signed binaries

##### Deliverables
- Enterprise customers can self-host or use managed cloud
- Full API for automation and integration
- AI-powered insights differentiate from all competitors

---

### 2.3 Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Agent** | Rust (existing TUI) | Already built, performant, cross-platform |
| **API Server** | Rust (Axum) | Same language as agent, excellent performance, shared types |
| **Database** | TimescaleDB (PostgreSQL) | Purpose-built for time-series, automatic partitioning, mature ecosystem |
| **Cache/Sessions** | Redis | Session storage, rate limiting, pub/sub for WebSocket fan-out |
| **Object Storage** | S3 / MinIO | PCAP file storage, works in cloud and self-hosted |
| **Frontend** | Next.js + TypeScript + Tailwind + shadcn/ui | Modern React stack, SSR for SEO, excellent DX |
| **Charts** | Tremor / Recharts | Time-series visualization, responsive, React-native |
| **Auth** | JWT + OAuth2 (via `oauth2` crate) | Standard, stateless, works with all SSO providers |
| **Payments** | Stripe | Industry standard, excellent API, handles tax/invoicing |
| **Email** | Resend | Developer-friendly transactional email, good deliverability |
| **Deployment** | Fly.io → AWS (at scale) | Start cheap and simple, migrate when economics justify |
| **CI/CD** | GitHub Actions | Already in use for the TUI |
| **Monitoring** | Sentry + own NetWatch | Eat your own dog food |

### 2.4 Data Schema (TimescaleDB)

```sql
-- Core metrics hypertable
CREATE TABLE host_metrics (
    time         TIMESTAMPTZ NOT NULL,
    host_id      UUID NOT NULL,
    metric_name  TEXT NOT NULL,        -- e.g., 'bandwidth_rx', 'latency_gateway'
    metric_value DOUBLE PRECISION,
    tags         JSONB                 -- e.g., {"interface": "eth0", "target": "8.8.8.8"}
);
SELECT create_hypertable('host_metrics', 'time');

-- Connections snapshots
CREATE TABLE connection_snapshots (
    time         TIMESTAMPTZ NOT NULL,
    host_id      UUID NOT NULL,
    process_name TEXT,
    pid          INTEGER,
    protocol     TEXT,
    state        TEXT,
    local_addr   TEXT,
    remote_addr  TEXT,
    remote_geo   JSONB
);
SELECT create_hypertable('connection_snapshots', 'time');

-- Alert events
CREATE TABLE alert_events (
    time         TIMESTAMPTZ NOT NULL,
    alert_rule_id UUID NOT NULL,
    host_id      UUID NOT NULL,
    state        TEXT NOT NULL,        -- 'firing', 'resolved'
    metric_value DOUBLE PRECISION,
    notified     BOOLEAN DEFAULT FALSE
);

-- PCAP metadata
CREATE TABLE pcap_files (
    id           UUID PRIMARY KEY,
    host_id      UUID NOT NULL,
    org_id       UUID NOT NULL,
    s3_key       TEXT NOT NULL,
    size_bytes   BIGINT,
    packet_count INTEGER,
    bpf_filter   TEXT,
    created_at   TIMESTAMPTZ NOT NULL,
    expires_at   TIMESTAMPTZ
);
```

### 2.5 Agent–Cloud Protocol

Communication uses **WebSocket** with MessagePack-serialized frames for efficiency:

```
Agent                                   Cloud
  │                                       │
  │──── AUTH (api_key) ──────────────────►│
  │◄─── AUTH_OK (host_id, config) ───────│
  │                                       │
  │──── SNAPSHOT (metrics bundle) ───────►│  (every 10s)
  │──── HEARTBEAT ───────────────────────►│  (every 30s)
  │                                       │
  │◄─── COMMAND (start_capture, ...) ────│  (on-demand)
  │──── CAPTURE_DATA (pcap chunk) ───────►│
  │──── CAPTURE_DONE ────────────────────►│
  │                                       │
  │◄─── CONFIG_UPDATE (new settings) ────│  (when user changes settings in web UI)
```

### 2.6 Security Considerations

| Concern | Mitigation |
|---------|------------|
| Agent auth | API keys are bcrypt-hashed server-side; transmitted over TLS only |
| Data in transit | All agent↔cloud communication over WSS (TLS 1.3) |
| Data at rest | PCAPs encrypted at rest in S3 (SSE-S3); database on encrypted volumes |
| API auth | JWT with short expiry (15 min) + refresh tokens (7 days) |
| Rate limiting | Per-agent and per-user rate limits; abuse triggers automatic block |
| PCAP sensitivity | PCAPs may contain credentials — warn users, auto-expire, allow immediate deletion |
| Multi-tenancy | All queries scoped by org_id; row-level security in PostgreSQL |
| Secrets management | Environment variables in production; no secrets in code or config files |
| Dependency supply chain | `cargo audit` in CI; Dependabot alerts enabled |

---

## Part 3: Immediate Next Steps (This Week)

1. **Refactor `netwatch` into a workspace** with `netwatch-core` (lib) and `netwatch-tui` (bin) crates — this unblocks sharing types with the API server
2. **Define the protobuf/MessagePack schemas** for the 5 snapshot types
3. **Add `--agent` flag** to main.rs for headless mode
4. **Scaffold the Axum API server** in a new `netwatch-cloud/` directory with basic ingest endpoint
5. **Set up Stripe account** and create product/price objects for the 4 paid tiers
6. **Register domain:** `netwatch.dev` or `netwatchapp.com`
7. **Create landing page:** Single-page marketing site with waitlist signup

---

## Part 4: Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Low free-to-paid conversion | Medium | High | Focus on alert features (hard to replicate locally); offer generous free trial |
| Enterprise competitors drop pricing | Low | Medium | Compete on simplicity and developer experience, not feature parity |
| Agent requires root/sudo | Medium | Medium | Offer non-root mode with graceful degradation; document capabilities per privilege level |
| PCAP storage costs at scale | Medium | Medium | Aggressive auto-expiry; charge for storage overage; compress with zstd |
| Solo founder burnout | High | Critical | Automate everything; keep scope minimal per phase; charge early to validate demand before building |
| Security breach / data leak | Low | Critical | SOC 2 prep from day 1; minimal data collection; encrypt everything; bug bounty program |

---

## Summary Timeline

```
Month  1–2   ████ Phase 0: Foundation (agent mode + API skeleton)
Month  2–4   ████████ Phase 1: Web Dashboard MVP (launch Starter/Pro)
Month  5–6   ██████ Phase 2: Alerting & Notifications
Month  7–9   ██████████ Phase 3: Packet Capture Cloud Features
Month  9–12  ████████████ Phase 4: Team & Collaboration (launch Team tier)
Month 12–18  ████████████████████████ Phase 5: Enterprise & Scale
```

**Target:** First paying customer by Month 3. $2,900 MRR by Month 12. $29,000 MRR by Month 24.
