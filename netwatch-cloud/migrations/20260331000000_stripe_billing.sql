-- Set trial_ends_at for existing accounts that don't have it
UPDATE accounts SET trial_ends_at = created_at + INTERVAL '14 days' WHERE trial_ends_at IS NULL;

-- Add subscription tracking
ALTER TABLE accounts ADD COLUMN IF NOT EXISTS stripe_subscription_id TEXT;
