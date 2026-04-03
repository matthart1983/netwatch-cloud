-- High Priority Security Fixes (Issues 7-15)

-- Issue #8: Add UNIQUE constraint for deduplication on (host_id, time)
-- Clean up any historical duplicates first so the constraint can be added safely.
DELETE FROM snapshots older
USING snapshots newer
WHERE older.host_id = newer.host_id
  AND older.time = newer.time
  AND older.id < newer.id;

-- This prevents duplicate snapshots from the same host at the same time
ALTER TABLE snapshots ADD CONSTRAINT unique_host_time UNIQUE(host_id, time);

-- Issue #12: Store alert state in database for persistence across restarts
CREATE TABLE alert_state (
    rule_id UUID NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    host_id UUID NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'ok',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (rule_id, host_id)
);

CREATE INDEX idx_alert_state_updated ON alert_state(updated_at DESC);

-- Create/update indexes for better query performance on alert state lookups
CREATE INDEX idx_alert_state_rule ON alert_state(rule_id);
CREATE INDEX idx_alert_state_host ON alert_state(host_id);
