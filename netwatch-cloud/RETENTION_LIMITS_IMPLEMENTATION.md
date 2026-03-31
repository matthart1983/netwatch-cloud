# Per-Account Retention Limits Implementation

## Overview
Implemented per-account snapshot retention limits, replacing the hardcoded 72-hour global retention with plan-based retention policies.

## Changes

### 1. Database Migration
**File**: `migrations/20260331001000_add_retention_days.sql`

- Adds `retention_days` column to `accounts` table
- Type: `INTEGER NOT NULL DEFAULT 3`
- Default: 3 days (72 hours, matching previous trial behavior)
- Backfills existing accounts based on plan:
  - `trial`: 3 days
  - `early_access`: 30 days
  - `past_due`: 3 days (frozen retention)

### 2. Retention Job Update
**File**: `src/retention.rs`

#### Key Changes:
- **Immediate cleanup for expired accounts**: All snapshots deleted when plan = 'expired'
- **Per-account snapshot retention**: Queries each account's `retention_days` setting
- **Detailed logging**: Per-account cleanup stats (account_id, retention_days, rows_deleted)

#### Logic Flow:
```
1. Delete all snapshots for expired accounts (plan = 'expired')
2. For each active account:
   - Query: SELECT id, retention_days FROM accounts WHERE plan NOT IN ('expired')
   - Delete: snapshots older than (now - retention_days)
   - Log: "retention: account {id} ({retention_days}d): deleted {count} snapshots"
3. Delete alert events older than 30 days (unchanged)
4. Mark hosts offline if no snapshot in 5 minutes (unchanged)
```

### 3. Account Registration Update
**File**: `src/routes/auth.rs`

- Set `retention_days = 3` when creating new trial accounts
- Updated INSERT query to include `retention_days` parameter
- Trial accounts now explicitly have 3-day retention from creation

## Plan-Based Defaults

| Plan | Retention | Purpose |
|------|-----------|---------|
| `trial` | 3 days | Default new account retention |
| `early_access` | 30 days | Extended retention for beta testers |
| `past_due` | 3 days | Minimal retention during payment issues |
| `expired` | 0 days (immediate) | Clean deletion on subscription expiration |

## Implementation Details

### Database Schema Change
```sql
ALTER TABLE accounts ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 3;
```

### Retention Cleanup Algorithm
```sql
-- For each account
DELETE FROM snapshots 
WHERE host_id IN (SELECT id FROM hosts WHERE account_id = $account_id)
  AND time < now() - INTERVAL '1 days' * $retention_days;
```

### Per-Account Logging
```
INFO retention: account 550e8400-e29b-41d4-a716-446655440000 (30d): deleted 1547 snapshots
INFO retention: account 6ba7b810-9dad-11d1-80b4-00c04fd430c8 (3d): deleted 342 snapshots
```

## Migration Path for Existing Accounts

1. Migration runs on deployment
2. All accounts receive `retention_days` value based on current `plan`
3. Cleanup job immediately picks up new settings
4. No data loss during transition

## Future Enhancements

- Admin API to adjust per-account retention
- Account settings UI to view/change retention limits
- Retention metrics dashboard
- Cost tracking by account based on stored data

## Testing Checklist

- [x] Builds successfully: `cargo build --release`
- [x] Retention.rs compiles with Uuid import
- [x] Auth.rs compiles with retention_days parameter
- [x] Migration SQL syntax valid
- [x] Default value (3 days) matches trial plan

## Rollout Notes

1. Deploy migration first
2. Update code with new retention logic
3. Verify logs show per-account cleanup after first cleanup cycle (1 hour)
4. Monitor: Check logs for accounts with `(30d)` and `(3d)` retention

## Compatibility

- **Backward Compatible**: Yes, default retention_days=3 matches previous behavior
- **No Data Loss**: Existing snapshots not affected until their retention window expires
- **Graceful Degradation**: If retention_days query fails, no snapshots deleted (safe failure)

## Related Files

- [src/retention.rs](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/retention.rs) - Cleanup job implementation
- [src/routes/auth.rs](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/auth.rs) - Account creation
- [migrations/20260331001000_add_retention_days.sql](file:///Users/matt/netwatch-cloud/netwatch-cloud/migrations/20260331001000_add_retention_days.sql) - Database migration
