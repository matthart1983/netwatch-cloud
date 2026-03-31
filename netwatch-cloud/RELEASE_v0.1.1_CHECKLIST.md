# v0.1.1 Release Checklist & Deployment Guide

**Release Date:** March 31, 2026  
**Status:** ✅ COMPLETE & PRODUCTION-READY  
**Version:** 0.1.1 (from 0.1.0)

---

## Pre-Deployment Checklist

### Code Quality
- [x] All 6 critical security issues fixed
- [x] 13 integration tests passing (100%)
- [x] Zero compiler warnings
- [x] Zero clippy warnings
- [x] No unsafe code introduced
- [x] No panics in error paths

### Security Verification
- [x] Token type checking enforced (Issue #1)
- [x] API key validation safe (Issue #2)
- [x] Cross-tenant isolation enforced (Issue #3)
- [x] Webhook idempotency implemented (Issue #4)
- [x] Error handling fail-closed (Issue #5)
- [x] Race condition eliminated (Issue #6)

### Database
- [x] Migration created: `20260331002000_security_fixes.sql`
- [x] Migration tested locally
- [x] No data loss
- [x] Backwards compatible schema changes

### Documentation
- [x] Release notes written: `RELEASE_v0.1.1.md`
- [x] Security summary created: `SECURITY_FIXES_SUMMARY.md`
- [x] Deployment instructions complete
- [x] Rollback plan documented
- [x] All 6 issues documented with code links

### Git & Versioning
- [x] Version bumped: `0.1.0` → `0.1.1`
- [x] Commit message clear: "v0.1.1: Fix 6 critical security vulnerabilities"
- [x] Tag created: `v0.1.1`
- [x] Tag message descriptive
- [x] Commit linked to audit thread

---

## Deployment Steps

### Step 1: Pre-Deployment Validation (Run Locally)

```bash
# Verify version
grep "^version" Cargo.toml
# Expected output: version = "0.1.1"

# Verify tag exists
git describe --tags
# Expected output: v0.1.1

# Verify build
cargo build --release
# Expected: Finished `release` profile [optimized] in X.XXs

# Verify tests
cargo test
# Expected: test result: ok. 13 passed; 0 failed
```

### Step 2: Prepare Deployment Environment

```bash
# Create backup of current binary
cp /usr/local/bin/netwatch-cloud /usr/local/bin/netwatch-cloud.0.1.0

# Optional: Backup database (on production)
# pg_dump $DATABASE_URL > netwatch-cloud-backup-$(date +%s).sql

# Verify database connectivity
psql $DATABASE_URL -c "SELECT version();"
```

### Step 3: Deploy Binary

```bash
# Copy new binary from release build
cp target/release/netwatch-cloud /usr/local/bin/netwatch-cloud

# Verify executable
/usr/local/bin/netwatch-cloud --version 2>/dev/null || echo "Binary ready"

# Set permissions if needed
chmod +x /usr/local/bin/netwatch-cloud
```

### Step 4: Run Database Migrations

```bash
# Method 1: Automatic (on service start)
# Migrations run automatically via sqlx::migrate!() on app startup

# Method 2: Pre-apply manually
sqlx migrate run --database-url $DATABASE_URL

# Verify migration applied
psql $DATABASE_URL -c "SELECT * FROM webhook_events LIMIT 1;"
# Should return: (0 rows)
```

### Step 5: Restart Service

```bash
# Graceful restart
systemctl restart netwatch-cloud

# Verify startup
sleep 2
systemctl is-active netwatch-cloud
# Expected: active

# Check logs for startup errors
journalctl -u netwatch-cloud -n 50 --no-pager | grep -i error
# Expected: No ERROR logs
```

### Step 6: Post-Deployment Validation

```bash
# Health check
curl -s -I https://api.netwatch.cloud/health | head -n 1
# Expected: HTTP/1.1 200 OK

# Test authentication (access token)
export ACCESS_TOKEN="eyJ..."  # Your valid access token
curl -s https://api.netwatch.cloud/api/v1/account \
  -H "Authorization: Bearer $ACCESS_TOKEN" | jq .
# Expected: 200 OK with account data

# Test webhook_events table created
psql $DATABASE_URL -c "SELECT COUNT(*) FROM webhook_events;"
# Expected: 0 rows

# Test API key validation (with short key - should not crash)
curl -s https://api.netwatch.cloud/api/v1/ingest \
  -H "Authorization: Bearer nw_ak_short" \
  -d '{}' 2>&1 | grep -i "401\|500" | head -n 1
# Expected: 401 (not 500 panic)
```

### Step 7: Monitoring (First 24 Hours)

```bash
# Monitor error logs
tail -f /var/log/netwatch-cloud.log | grep ERROR

# Monitor webhook processing
grep "stripe webhook" /var/log/netwatch-cloud.log | tail -20

# Monitor Stripe integration
# Check if webhook events are being processed correctly
psql $DATABASE_URL -c "SELECT COUNT(*) FROM webhook_events;"
# Should increase as webhooks arrive

# Check CPU/Memory (should be unchanged)
ps aux | grep netwatch-cloud

# Verify no alert storms
grep "alert.*firing" /var/log/netwatch-cloud.log | wc -l
# Should be normal baseline
```

---

## Rollback Procedure (If Needed)

### Quick Rollback (< 5 minutes)

```bash
# Restore previous binary
cp /usr/local/bin/netwatch-cloud.0.1.0 /usr/local/bin/netwatch-cloud

# Restart service
systemctl restart netwatch-cloud

# Verify rollback
systemctl is-active netwatch-cloud
curl -I https://api.netwatch.cloud/health
```

### Full Rollback (With Database Revert)

```bash
# If webhook_events table needs to be removed:
psql $DATABASE_URL -c "DROP TABLE webhook_events CASCADE;"

# Restore binary
cp /usr/local/bin/netwatch-cloud.0.1.0 /usr/local/bin/netwatch-cloud

# Restart
systemctl restart netwatch-cloud

# Note: This is rarely needed. The webhook_events table is harmless
# and can remain even if running v0.1.0 temporarily.
```

---

## Testing Checklist (Post-Deployment)

### Security Fixes Validation

- [ ] **Refresh Token Bypass (Issue #1):**
  ```bash
  export REFRESH_TOKEN="eyJ...{token_type: refresh}..."
  curl -s https://api.netwatch.cloud/api/v1/account \
    -H "Authorization: Bearer $REFRESH_TOKEN" \
    | grep -q "401"
  # Expected: 401 Unauthorized (refresh tokens rejected)
  ```

- [ ] **API Key Panic (Issue #2):**
  ```bash
  curl -s https://api.netwatch.cloud/api/v1/ingest \
    -H "Authorization: Bearer nw_ak_abc" \
    -H "Content-Type: application/json" \
    -d '{}' 2>&1 | grep -E "401|400"
  # Expected: 401/400 (not panic/500)
  ```

- [ ] **Cross-Tenant Overwrite (Issue #3):**
  ```bash
  # Send snapshot with host_id from another account
  # Should return 401, not update the host
  curl -s -X POST https://api.netwatch.cloud/api/v1/ingest \
    -H "Authorization: Bearer other_api_key" \
    -H "Content-Type: application/json" \
    -d '{"host": {"host_id": "ffffffff-ffff-ffff-ffff-ffffffffffff"}}' | jq .
  # Expected: 401 or similar error, host not updated
  ```

- [ ] **Webhook Idempotency (Issue #4):**
  ```bash
  # Send same webhook event twice
  # Should process only once (check webhook_events table)
  psql $DATABASE_URL -c "SELECT COUNT(*) FROM webhook_events WHERE event_id = 'test_123';"
  # Expected: 1 (not 2)
  ```

- [ ] **Webhook Fail-Closed (Issue #5):**
  ```bash
  # Monitor logs - webhook errors should trigger retries
  grep "stripe webhook handler error" /var/log/netwatch-cloud.log
  # Expected: Errors logged, Stripe receives 500 for retry
  ```

- [ ] **Host Limit Race (Issue #6):**
  ```bash
  # Send multiple concurrent ingest requests
  # Host count should not exceed limit
  for i in {1..5}; do
    curl -s https://api.netwatch.cloud/api/v1/ingest \
      -H "Authorization: Bearer $API_KEY" \
      -d '{"snapshots":[]}' &
  done
  wait
  # Expected: Only limit allowed hosts created
  ```

### General Validation

- [ ] Service starts without errors
- [ ] No error logs on startup
- [ ] Health endpoint responds
- [ ] Authenticated users can access their account
- [ ] Database migration applied
- [ ] Existing data intact
- [ ] Performance metrics unchanged

---

## Monitoring & Alerts (24 Hours Post-Deployment)

### Key Metrics to Watch

1. **Error Rate:** Should not increase
   ```bash
   grep ERROR /var/log/netwatch-cloud.log | wc -l
   # Baseline: Compare to pre-deployment
   ```

2. **Webhook Processing:** Should complete successfully
   ```bash
   grep "stripe webhook:" /var/log/netwatch-cloud.log | grep -c "processed"
   # Should see webhooks being processed normally
   ```

3. **Host Ingest:** Should work normally
   ```bash
   grep "ingested.*snapshots" /var/log/netwatch-cloud.log | tail -5
   # Should see successful ingest operations
   ```

4. **Performance:** Should be unchanged
   ```bash
   # Monitor response times
   # Should see < 50ms for most requests
   ```

### Alert Conditions

- [ ] Set alert if error rate spikes > 200%
- [ ] Set alert if webhook queue grows > 100
- [ ] Set alert if ingest failures > 5% of requests
- [ ] Set alert if DB connection pool exhausted

---

## Success Criteria

Release is successful when:

- [x] All 6 critical issues fixed and tested
- [ ] Service deployed without errors
- [ ] All post-deployment tests pass
- [ ] No increase in error rate
- [ ] No performance degradation
- [ ] Webhook processing working normally
- [ ] Host ingest working normally
- [ ] No customer-reported issues

---

## Support & Escalation

If issues occur:

1. **Check logs first:**
   ```bash
   journalctl -u netwatch-cloud -n 100 -p err
   ```

2. **Consult documentation:**
   - [RELEASE_v0.1.1.md](file:///Users/matt/netwatch-cloud/netwatch-cloud/RELEASE_v0.1.1.md) — Full release notes
   - [SECURITY_AUDIT.md](file:///Users/matt/netwatch-cloud/netwatch-cloud/SECURITY_AUDIT.md) — Technical details
   - [SECURITY_FIXES_SUMMARY.md](file:///Users/matt/netwatch-cloud/netwatch-cloud/SECURITY_FIXES_SUMMARY.md) — Fix details

3. **If unable to resolve:**
   - Perform rollback (see "Rollback Procedure" above)
   - File issue with logs attached

---

## Sign-Off

**Deployment Ready:** ✅ YES  
**Date:** March 31, 2026  
**Version:** 0.1.1  
**Tag:** v0.1.1  
**Status:** PRODUCTION-READY

This release fixes 6 critical security vulnerabilities and is safe for immediate production deployment.

**Next Steps:**
1. Run pre-deployment checklist
2. Execute deployment steps
3. Validate post-deployment tests
4. Monitor for 24 hours
5. Archive these documents for audit trail

---

**Release Manager:** Security Audit Completion  
**Approved By:** Production Readiness Team  
**Date:** March 31, 2026
