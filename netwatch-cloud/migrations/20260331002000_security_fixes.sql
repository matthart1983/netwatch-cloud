-- Security fixes for v0.1.1
-- Issue #4: Webhook idempotency tracking
CREATE TABLE webhook_events (
    event_id        TEXT PRIMARY KEY,
    event_type      TEXT NOT NULL,
    processed_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Issue #3: Add composite unique constraint for cross-tenant host protection
-- Note: We'll add account_id to the conflict resolution in the upsert logic
CREATE UNIQUE INDEX idx_hosts_id_account ON hosts(id, account_id);
