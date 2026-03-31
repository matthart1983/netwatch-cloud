-- Add per-account retention limits
ALTER TABLE accounts ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 3;

-- Set retention_days based on plan for existing accounts
UPDATE accounts SET retention_days = 3 WHERE plan = 'trial';
UPDATE accounts SET retention_days = 30 WHERE plan = 'early_access';
UPDATE accounts SET retention_days = 3 WHERE plan = 'past_due';
