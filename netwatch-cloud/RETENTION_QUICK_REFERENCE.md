# Per-Account Retention Limits - Quick Reference

## What Changed

Snapshot retention moved from hardcoded 72 hours to per-account settings based on subscription plan.

## Files Modified

| File | Changes |
|------|---------|
| `migrations/20260331001000_add_retention_days.sql` | New migration adding `retention_days` column |
| `src/retention.rs` | Updated cleanup job for per-account retention |
| `src/routes/auth.rs` | Set `retention_days` on account creation |

## Migration Details

```sql
ALTER TABLE accounts ADD COLUMN retention_days INTEGER NOT NULL DEFAULT 3;
```

Backfills existing accounts:
- `trial` → 3 days
- `early_access` → 30 days  
- `past_due` → 3 days
- `expired` → 0 days (immediate cleanup)

## Cleanup Job Changes

**Before:**
```sql
DELETE FROM snapshots WHERE time < now() - INTERVAL '72 hours'
```

**After:**
```sql
-- For each account
DELETE FROM snapshots 
WHERE host_id IN (SELECT id FROM hosts WHERE account_id = $1)
  AND time < now() - INTERVAL '1 days' * $retention_days
```

**Per-Account Logging:**
```
INFO retention: account 550e8400-e29b-41d4-a716-446655440000 (30d): deleted 1547 snapshots
```

## Testing

```bash
# Build check
cargo check

# Release build
cargo build --release
```

Both pass successfully ✓

## Deployment

1. Run migration (auto-applied on startup)
2. Deploy new code
3. Check logs for per-account cleanup messages after 1 hour

## Admin Operations

### View account retention
```sql
SELECT id, email, plan, retention_days FROM accounts;
```

### Update account retention (manual)
```sql
UPDATE accounts SET retention_days = 60 WHERE plan = 'early_access';
```

### Check upcoming deletions
```sql
SELECT a.id, a.email, a.retention_days, 
       COUNT(s.id) as old_snapshots
FROM accounts a
LEFT JOIN hosts h ON h.account_id = a.id
LEFT JOIN snapshots s ON s.host_id = h.id
WHERE s.time < now() - INTERVAL '1 days' * a.retention_days
GROUP BY a.id;
```

## Plan Retention Matrix

| Plan | Days | Use Case |
|------|------|----------|
| trial | 3 | Default new accounts (72 hours) |
| early_access | 30 | Extended retention for beta |
| past_due | 3 | Payment-failed minimal retention |
| expired | 0 | Subscription ended, immediate deletion |

## Known Behavior

- Cleanup job runs every 60 minutes
- Snapshots deleted in batches per-account
- Alert events still deleted after 30 days (independent)
- Hosts marked offline after 5 minutes inactivity (unchanged)
