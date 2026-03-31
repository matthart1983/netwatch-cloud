-- FIX #4: Add webhook_events table for idempotency
-- Track webhook events by ID to prevent duplicate processing
CREATE TABLE IF NOT EXISTS webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    processed BOOLEAN NOT NULL DEFAULT false
);

-- Create index for lookups
CREATE INDEX IF NOT EXISTS idx_webhook_events_processed 
ON webhook_events(processed, created_at DESC);

-- Index for cleanup queries
CREATE INDEX IF NOT EXISTS idx_webhook_events_created_at 
ON webhook_events(created_at DESC);
