-- Medium Priority Security Fixes (Issues 16-20)

-- Issue #19: Add CHECK constraints to enforce valid database states
-- Prevents invalid plan values
ALTER TABLE accounts ADD CONSTRAINT valid_plan 
  CHECK (plan IN ('trial', 'early_access', 'past_due', 'expired'));

-- Enforce retention days within valid range (1-730 days)
ALTER TABLE accounts ADD CONSTRAINT valid_retention_days 
  CHECK (retention_days >= 1 AND retention_days <= 730);

-- Ensure trial accounts have an expiry date
ALTER TABLE accounts ADD CONSTRAINT trial_requires_expiry 
  CHECK (plan != 'trial' OR trial_ends_at IS NOT NULL);

-- Ensure API key prefixes are unique (additional constraint for security)
ALTER TABLE api_keys ADD CONSTRAINT unique_key_prefix 
  UNIQUE(key_prefix);

-- Validate snapshot timestamps are reasonable (not in distant future)
ALTER TABLE snapshots ADD CONSTRAINT valid_snapshot_time
  CHECK (time >= '2020-01-01' AND time <= now() + INTERVAL '1 day');
