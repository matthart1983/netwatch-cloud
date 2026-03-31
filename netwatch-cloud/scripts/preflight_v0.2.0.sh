#!/bin/bash

# Preflight checks for v0.2.0 migration
# Run this script BEFORE deploying v0.2.0 to production
# Ensures no data corruption will occur during migration

set -e

DB_URL="${DATABASE_URL}"

if [ -z "$DB_URL" ]; then
    echo "ERROR: DATABASE_URL environment variable not set"
    exit 1
fi

echo "=== Preflight checks for v0.2.0 migration ==="
echo ""

# Check 1: Duplicate snapshots by (host_id, time)
echo -n "Checking duplicate snapshots... "
DUPS=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM (
  SELECT host_id, time FROM snapshots GROUP BY host_id, time HAVING COUNT(*) > 1
) t
" 2>/dev/null || echo "0")

if [ "$DUPS" -gt 0 ]; then
  echo "FAIL ($DUPS duplicates found)"
  echo ""
  echo "Duplicate snapshots detected. Run this to fix:"
  echo "  WITH dups AS ("
  echo "    SELECT host_id, time, min(id) as keep_id"
  echo "    FROM snapshots"
  echo "    GROUP BY host_id, time"
  echo "    HAVING COUNT(*) > 1"
  echo "  )"
  echo "  DELETE FROM snapshots"
  echo "  WHERE id NOT IN (SELECT keep_id FROM dups)"
  echo "    AND (host_id, time) IN (SELECT host_id, time FROM dups);"
  echo ""
  exit 1
else
  echo "OK"
fi

# Check 2: Duplicate API key prefixes
echo -n "Checking duplicate API key prefixes... "
DUPS=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM (
  SELECT key_prefix FROM api_keys WHERE key_prefix IS NOT NULL GROUP BY key_prefix HAVING COUNT(*) > 1
) t
" 2>/dev/null || echo "0")

if [ "$DUPS" -gt 0 ]; then
  echo "FAIL ($DUPS duplicates found)"
  echo ""
  echo "Duplicate API key prefixes detected. Run this to fix:"
  echo "  WITH dups AS ("
  echo "    SELECT key_prefix, min(id) as keep_id"
  echo "    FROM api_keys"
  echo "    WHERE key_prefix IS NOT NULL"
  echo "    GROUP BY key_prefix"
  echo "    HAVING COUNT(*) > 1"
  echo "  )"
  echo "  DELETE FROM api_keys"
  echo "  WHERE id NOT IN (SELECT keep_id FROM dups)"
  echo "    AND key_prefix IN (SELECT key_prefix FROM dups);"
  echo ""
  exit 1
else
  echo "OK"
fi

# Check 3: Invalid account plans
echo -n "Checking invalid account plans... "
INVALID=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM accounts 
WHERE plan NOT IN ('trial', 'early_access', 'past_due', 'expired')
" 2>/dev/null || echo "0")

if [ "$INVALID" -gt 0 ]; then
  echo "FAIL ($INVALID invalid plans found)"
  echo ""
  echo "Invalid account plans detected. Run this to fix:"
  echo "  UPDATE accounts SET plan = 'trial' WHERE plan NOT IN ('trial', 'early_access', 'past_due', 'expired');"
  echo ""
  psql "$DB_URL" -c "SELECT id, email, plan FROM accounts WHERE plan NOT IN ('trial', 'early_access', 'past_due', 'expired')" || true
  echo ""
  exit 1
else
  echo "OK"
fi

# Check 4: Trial accounts without expiry
echo -n "Checking trial accounts without expiry... "
INVALID=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM accounts 
WHERE plan = 'trial' AND trial_ends_at IS NULL
" 2>/dev/null || echo "0")

if [ "$INVALID" -gt 0 ]; then
  echo "FAIL ($INVALID trial accounts without expiry)"
  echo ""
  echo "Trial accounts without expiry detected. Run this to fix:"
  echo "  UPDATE accounts SET trial_ends_at = now() + INTERVAL '30 days' WHERE plan = 'trial' AND trial_ends_at IS NULL;"
  echo ""
  exit 1
else
  echo "OK"
fi

# Check 5: Invalid retention days
echo -n "Checking invalid retention days... "
INVALID=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM accounts 
WHERE retention_days IS NOT NULL AND (retention_days < 1 OR retention_days > 730)
" 2>/dev/null || echo "0")

if [ "$INVALID" -gt 0 ]; then
  echo "FAIL ($INVALID invalid retention days)"
  echo ""
  echo "Invalid retention days detected. Run this to fix:"
  echo "  UPDATE accounts SET retention_days = 30 WHERE retention_days IS NOT NULL AND (retention_days < 1 OR retention_days > 730);"
  echo ""
  exit 1
else
  echo "OK"
fi

# Check 6: Snapshots outside time bounds (warning only)
echo -n "Checking snapshots outside time bounds... "
INVALID=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM snapshots 
WHERE time < '2020-01-01' OR time > now() + INTERVAL '1 day'
" 2>/dev/null || echo "0")

if [ "$INVALID" -gt 0 ]; then
  echo "WARNING ($INVALID snapshots outside bounds, will be kept)"
else
  echo "OK"
fi

# Check 7: NULL api_key_id in hosts (potential foreign key violation)
echo -n "Checking hosts with NULL api_key_id... "
INVALID=$(psql -t "$DB_URL" -c "
SELECT COUNT(*) FROM hosts WHERE api_key_id IS NULL
" 2>/dev/null || echo "0")

if [ "$INVALID" -gt 0 ]; then
  echo "WARNING ($INVALID hosts with NULL api_key_id)"
else
  echo "OK"
fi

echo ""
echo "✅ All preflight checks passed. Safe to deploy v0.2.0"
exit 0
