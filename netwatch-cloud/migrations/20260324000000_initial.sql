CREATE TABLE accounts (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    notify_email    BOOLEAN NOT NULL DEFAULT true,
    slack_webhook   TEXT,
    stripe_customer_id TEXT,
    plan            TEXT NOT NULL DEFAULT 'trial',
    trial_ends_at   TIMESTAMPTZ
);

CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id      UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_hash        TEXT NOT NULL,
    key_prefix      TEXT NOT NULL,
    label           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_account ON api_keys(account_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);

CREATE TABLE hosts (
    id              UUID PRIMARY KEY,
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

CREATE TABLE snapshots (
    id              BIGSERIAL PRIMARY KEY,
    host_id         UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    time            TIMESTAMPTZ NOT NULL,
    connection_count INTEGER,
    gateway_ip      TEXT,
    gateway_rtt_ms  DOUBLE PRECISION,
    gateway_loss_pct DOUBLE PRECISION,
    dns_ip          TEXT,
    dns_rtt_ms      DOUBLE PRECISION,
    dns_loss_pct    DOUBLE PRECISION
);

CREATE INDEX idx_snapshots_host_time ON snapshots(host_id, time DESC);

CREATE TABLE interface_metrics (
    id              BIGSERIAL PRIMARY KEY,
    snapshot_id     BIGINT NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
    host_id         UUID NOT NULL,
    time            TIMESTAMPTZ NOT NULL,
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

CREATE TABLE alert_rules (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id      UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    host_id         UUID REFERENCES hosts(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    metric          TEXT NOT NULL,
    condition       TEXT NOT NULL,
    threshold       DOUBLE PRECISION,
    threshold_str   TEXT,
    duration_secs   INTEGER NOT NULL DEFAULT 60,
    severity        TEXT NOT NULL DEFAULT 'warning',
    enabled         BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_alert_rules_account ON alert_rules(account_id);

CREATE TABLE alert_events (
    id              BIGSERIAL PRIMARY KEY,
    rule_id         UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    host_id         UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    state           TEXT NOT NULL,
    metric_value    DOUBLE PRECISION,
    message         TEXT NOT NULL,
    notified        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_alert_events_rule ON alert_events(rule_id, created_at DESC);
CREATE INDEX idx_alert_events_host ON alert_events(host_id, created_at DESC);
