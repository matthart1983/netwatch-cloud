# NetWatch Cloud — Comprehensive Security & Design Audit

**Date:** March 31, 2026  
**Severity Levels:** Critical, High, Medium  
**Total Issues Found:** 20+ (6 critical, 8 high, 8+ medium)  
**Estimated Fix Time:** 1-2 days for critical/high items

---

## Executive Summary

**Good News:** SQL injection and classic web vulnerabilities are well-protected.  
**Key Risks:** Auth boundary failures, cross-tenant data isolation, webhook safety, and silent error handling that masks billing/alert drift.

**Recommendation:** Do a focused 1-2 day hardening pass on the 6 critical issues below. The current codebase is reasonably safe for a small SaaS but has design patterns that will break at scale or under concurrent load.

---

## Critical Issues (Fix Immediately)

### 1. ⚠️ CRITICAL: Refresh Tokens Accepted as Access Tokens

**Location:** [src/auth.rs#L50-L73](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/auth.rs#L50-L73)

**Issue:**
```rust
impl FromRequestParts<Arc<AppState>> for AuthUser {
    async fn from_request_parts(...) -> Result<Self, Self::Rejection> {
        let token = auth_header.strip_prefix("Bearer ").ok_or(...)?;
        let claims = verify_token(token, &state.config.jwt_secret)  // ← No token type check
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        Ok(AuthUser { account_id: claims.sub })
    }
}
```

A refresh token (valid for 7 days) can be used as a normal access token. If a refresh token is stolen or leaked, attackers have 7 days instead of 15 minutes.

**Impact:** Token lifetime extended 28x; account compromise window much larger.

**Fix:**
```rust
pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims, Error> {
    let claims = verify_token(token, secret)?;
    // Add to Claims: pub token_type: String  ("access" vs "refresh")
    if claims.token_type != "access" {
        return Err(TokenTypeError);
    }
    Ok(claims)
}

// In AuthUser extractor:
let claims = verify_access_token(token, &state.config.jwt_secret)?;
```

**Priority:** CRITICAL — Apply immediately

---

### 2. ⚠️ CRITICAL: Panic on Short API Keys

**Location:** [src/auth.rs#L99-L103](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/auth.rs#L99-L103)

**Issue:**
```rust
if api_key.len() < 12 || !api_key.starts_with("nw_ak_") {
    return Err(StatusCode::UNAUTHORIZED);
}
let prefix = &api_key[..14];  // ← Can panic if len() == 12 or 13
```

A 12- or 13-character API key passes the `len() < 12` check but causes a panic on the slice `[..14]`.

**Impact:** DoS via malformed API key; server crash; metrics not collected.

**Fix:**
```rust
if api_key.len() < 14 || !api_key.starts_with("nw_ak_") {
    return Err(StatusCode::UNAUTHORIZED);
}
let prefix = &api_key[..14];
```

**Priority:** CRITICAL — Apply immediately

---

### 3. ⚠️ CRITICAL: Cross-Tenant Host Overwrite

**Location:** [src/routes/ingest.rs#L73-L106](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/ingest.rs#L73-L106)

**Issue:**
```sql
INSERT INTO hosts (id, account_id, api_key_id, ...)
VALUES ($1, $2, $3, ...)
ON CONFLICT (id) DO UPDATE SET
    hostname = EXCLUDED.hostname,
    ...
```

The conflict is on `(id)` only. If an agent from Account B submits a snapshot with a `host_id` that belongs to Account A:
1. The `ON CONFLICT` triggers
2. It updates Account A's host metadata
3. Snapshot is attached to a host in another account
4. Monitoring data is corrupted / cross-tenant boundary is broken

**Impact:** **Data pollution**, forged monitoring data, account impersonation, alert misconfiguration.

**Example Attack:**
```
Account A owns host_id = uuid-123
Account B agent sends: { host_id: uuid-123, ... }
Result: Account A's host is updated with Account B's data + snapshots
```

**Fix:**
```rust
// Option 1: Reject if host exists for different account
let existing_account: Option<Uuid> = sqlx::query_scalar(
    "SELECT account_id FROM hosts WHERE id = $1"
)
.bind(host_id)
.fetch_optional(&state.db)
.await?;

if let Some(existing) = existing_account {
    if existing != agent.account_id {
        return Err(StatusCode::UNAUTHORIZED);  // Reject
    }
}

// Option 2: Make constraint include account_id (better)
// ALTER TABLE hosts ADD CONSTRAINT upsert_key UNIQUE(id, account_id);
// Then in ON CONFLICT, also check account_id
```

**Priority:** CRITICAL — This is a tenant boundary break. Fix before next deployment.

---

### 4. ⚠️ CRITICAL: No Webhook Idempotency / Replay Protection

**Location:** [src/routes/billing.rs#L150-L204](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/billing.rs#L150-L204)

**Issue:**
```rust
pub async fn stripe_webhook(...) -> StatusCode {
    // No event_id deduplication
    let event: serde_json::Value = serde_json::from_str(payload)?;
    
    match event_type {
        "customer.subscription.updated" => {
            handle_subscription_updated(data_object, &state).await
        }
        ...
    }
    StatusCode::OK
}
```

Stripe retries webhooks on timeout/failure. Without idempotency:
- Same event processed multiple times
- Billing state diverges (plan updated 3x for 1 subscription change)
- If you lose the DB write but return 200, Stripe thinks it's delivered and stops retrying

**Impact:** Silent billing drift, lost plan changes, revenue loss tracking.

**Fix:**
```rust
// Store event_id in DB with transaction
let event_id: String = event["id"].as_str().ok_or(...)?.to_string();

let already_processed: bool = sqlx::query_scalar(
    "SELECT EXISTS(SELECT 1 FROM webhook_events WHERE event_id = $1)"
)
.bind(&event_id)
.fetch_one(&state.db)
.await?;

if already_processed {
    return StatusCode::OK;  // Idempotent
}

// Process and insert record
sqlx::query("INSERT INTO webhook_events (event_id, event_type, ...) VALUES ...")
    .execute(&state.db)
    .await?;

// Only return 200 after DB commit
StatusCode::OK
```

**Priority:** CRITICAL — Billing events are sensitive.

---

### 5. ⚠️ CRITICAL: Webhooks Return 200 Even on Processing Failures

**Location:** [src/routes/billing.rs#L199-L203](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/billing.rs#L199-L203)

**Issue:**
```rust
if let Err(e) = result {
    tracing::error!("stripe webhook handler error: {}", e);
}

StatusCode::OK  // Always returns OK, even on error
```

Stripe treats `200 OK` as successful delivery and stops retrying. If the DB write fails, the event is lost forever.

**Impact:** Silent billing state corruption.

**Fix:**
```rust
match result {
    Ok(_) => StatusCode::OK,
    Err(e) => {
        tracing::error!("stripe webhook handler error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR  // Stripe will retry
    }
}
```

**Priority:** CRITICAL

---

### 6. ⚠️ CRITICAL: Host Limit Race Condition

**Location:** [src/routes/ingest.rs#L44-L70](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/ingest.rs#L44-L70)

**Issue:**
```rust
let host_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM hosts WHERE account_id = $1")
    .bind(agent.account_id)
    .fetch_one(&state.db)
    .await?;

// Two concurrent requests here can both pass the check
if host_count >= host_limit {
    if !host_exists { return Err(StatusCode::PAYMENT_REQUIRED); }
}

// Both requests proceed and upsert
sqlx::query("INSERT INTO hosts ...")
    .execute(&state.db)
    .await?;
```

Two simultaneous ingest requests can both see `COUNT(*) = 2` when limit is 3, both pass the check, and both create/upsert hosts.

**Impact:** Account exceeds host limit; usage enforcement fails.

**Fix:**
```rust
// Use transaction + lock
let mut tx = state.db.begin().await?;

let host_count: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM hosts WHERE account_id = $1 FOR UPDATE"  // Lock row
)
.bind(agent.account_id)
.fetch_one(&mut *tx)
.await?;

if host_count >= host_limit && !host_exists {
    return Err(StatusCode::PAYMENT_REQUIRED);
}

// Proceed with upsert inside transaction
sqlx::query("INSERT INTO hosts ...")
    .execute(&mut *tx)
    .await?;

tx.commit().await?;
```

**Priority:** CRITICAL (impacts billing enforcement)

---

## High-Priority Issues (Fix in Next 2-3 Days)

### 7. ⚠️ HIGH: Ingest Partial Writes Despite "Rejected" Response

**Location:** [src/routes/ingest.rs#L110-L207](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/ingest.rs#L110-L207)

**Issue:**
```rust
for snapshot in &payload.snapshots {
    let snapshot_id: i64 = sqlx::query_scalar(
        "INSERT INTO snapshots (...) VALUES (...) RETURNING id"
    )
    .fetch_one(&state.db)  // Inserted
    .await?;

    // If this fails, snapshot remains but response says rejected
    for iface in &snapshot.interfaces {
        sqlx::query("INSERT INTO interface_metrics ...")
            .execute(&state.db)
            .await?;  // ← Failure here leaves orphaned snapshot
    }
}
```

If snapshot inserts successfully but interface_metrics fails, the response reports rejection but partial data remains.

**Impact:** Orphaned snapshots; disk waste; inconsistent alert state.

**Fix:** Wrap each snapshot + children in a transaction:
```rust
let mut tx = state.db.begin().await?;

let snapshot_id = sqlx::query_scalar("INSERT INTO snapshots ... RETURNING id")
    .fetch_one(&mut *tx)
    .await?;

for iface in &snapshot.interfaces {
    sqlx::query("INSERT INTO interface_metrics ...")
        .execute(&mut *tx)
        .await?;  // ← If fails, entire snapshot is rolled back
}

tx.commit().await?;
accepted += 1;
```

**Priority:** HIGH

---

### 8. ⚠️ HIGH: No Idempotency / Deduplication for Ingest Retries

**Issue:** Agents will retry on network failures. Without a uniqueness constraint, duplicate snapshots are stored, distorting metrics and alerts.

**Example:**
- Agent sends snapshot at 2026-03-31 10:00:00
- Network timeout, agent retries
- Same snapshot inserted twice
- Alert evaluates duplicate data, fires twice

**Fix:** Add schema constraint:
```sql
ALTER TABLE snapshots ADD CONSTRAINT unique_host_time UNIQUE(host_id, time);
```

**Impact:** Prevents duplicate data; automatic dedupe on retry.

**Priority:** HIGH

---

### 9. ⚠️ HIGH: Untrusted Client Timestamps

**Location:** [src/routes/ingest.rs#L120](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/ingest.rs#L120)

**Issue:**
```rust
.bind(snapshot.timestamp)  // Trusted from agent; can be any time
```

Agent provides timestamp directly. If agent clock is wrong or compromised:
- Future timestamps break retention queries
- Past timestamps hide outages
- Alerts evaluate stale/wrong data

**Impact:** Unreliable monitoring and alerting.

**Fix:** Store both timestamps:
```rust
sqlx::query(
    "INSERT INTO snapshots (host_id, reported_at, received_at, ...) 
     VALUES ($1, $2, now(), ...)"
)
.bind(snapshot.timestamp)  // reported_at
.execute(&state.db)
.await?;

// Also validate: reported_at within ±1 hour of now()
let skew = (Utc::now() - snapshot.timestamp).num_seconds().abs();
if skew > 3600 {
    return Err("Timestamp skew too large");
}
```

**Priority:** HIGH

---

### 10. ⚠️ HIGH: Webhook Signature Verification is Fail-Open

**Location:** [src/routes/billing.rs#L166-L171](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/billing.rs#L166-L171)

**Issue:**
```rust
if let Some(ref secret) = state.config.stripe_webhook_secret {
    if !verify_signature(payload, sig_header, secret) {
        return StatusCode::BAD_REQUEST;
    }
} else {
    // No secret configured? Accept all webhooks!
    tracing::warn!("stripe webhook: signature format valid, ...");
}
```

If `STRIPE_WEBHOOK_SECRET` is not set, **all webhook requests are accepted**, including forged ones.

**Impact:** Attacker can update billing status for any account.

**Fix:** Fail startup or fail all webhook requests if secret is missing:
```rust
pub async fn stripe_webhook(...) -> Result<StatusCode, StatusCode> {
    let secret = state.config.stripe_webhook_secret
        .as_ref()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;  // Fail if missing
    
    if !verify_signature(payload, sig_header, secret) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    // Process ...
    Ok(StatusCode::OK)
}
```

Also: add startup check:
```rust
if cfg!(feature = "stripe_billing") && config.stripe_webhook_secret.is_none() {
    panic!("Stripe webhook secret required when stripe_billing is enabled");
}
```

**Priority:** HIGH (billing security)

---

### 11. ⚠️ HIGH: Slack Webhook URL Exposed in API Response

**Location:** [src/routes/billing.rs#L32-L60](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/billing.rs#L32-L60)

**Issue:**
```rust
pub struct AccountInfo {
    pub slack_webhook: Option<String>,  // Full URL returned
}

pub async fn get_account(...) -> Result<Json<AccountInfo>, StatusCode> {
    // slack_webhook is included in JSON response
}
```

The full Slack webhook URL is a bearer token. Returning it in API responses:
- Exposes it in logs, browser history, proxy logs
- Makes it available in paginated/cached responses
- Increases compromise surface

**Impact:** Attacker can intercept webhook URL and POSTs to attacker's server.

**Fix:**
```rust
pub struct AccountInfo {
    pub slack_webhook_configured: bool,    // Not the URL itself
    pub slack_webhook_last_4: Option<String>,  // Last 4 chars, if set
}

pub async fn get_account(...) -> Result<Json<AccountInfo>, StatusCode> {
    let slack_configured = slack_webhook.is_some();
    let last_4 = slack_webhook.as_ref().map(|w| {
        if w.len() >= 4 { w[w.len()-4..].to_string() } else { "****".to_string() }
    });
    
    Ok(Json(AccountInfo {
        slack_webhook_configured,
        slack_webhook_last_4: last_4,
        ...
    }))
}
```

Also validate Slack URLs at write time:
```rust
if let Some(ref webhook) = req.slack_webhook {
    if !webhook.starts_with("https://hooks.slack.com/") {
        return Err(StatusCode::BAD_REQUEST);
    }
}
```

**Priority:** HIGH (secret exposure)

---

### 12. ⚠️ HIGH: Alert State Lost on Restart

**Location:** [src/alerts/engine.rs#L23](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/alerts/engine.rs#L23)

**Issue:**
```rust
let mut states: HashMap<StateKey, AlertState> = HashMap::new();

loop {
    ticker.tick().await;
    evaluate_cycle(&state, &mut states).await;
    // If server restarts, states HashMap is lost
}
```

Alert state is only in memory. On server restart:
- All pending timers are lost
- Alerts reset to "Ok"
- Previously-firing alerts may not retrigger
- Duplicate notifications on next cycle

**Impact:** Inconsistent alert behavior; missed notifications.

**Fix Option 1 (Simple, acceptable for now):** Log that alert state was lost:
```rust
info!("Alert engine starting. Previous state lost if server restarted.");
// Alerts will re-evaluate fresh; some may retrigger unnecessarily.
// This is acceptable for small deployments.
```

**Fix Option 2 (Better):** Persist state in DB (more involved).

**Priority:** HIGH (affects reliability, but acceptable for single-instance)

---

### 13. ⚠️ HIGH: Duplicate Alert Evaluations on Multi-Instance Deployments

**Location:** [src/alerts/engine.rs](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/alerts/engine.rs) and [src/retention.rs](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/retention.rs)

**Issue:** Both background jobs run on every instance without coordination. If you deploy 2 replicas:
- Alert engine runs 2x → duplicate alert notifications
- Retention cleanup runs 2x → DB contention

**Fix:** Use Postgres advisory lock:
```rust
pub async fn run(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(30));
    let lock_id: i64 = 42;  // Arbitrary unique ID per job
    
    loop {
        ticker.tick().await;
        
        // Try to acquire lock (non-blocking)
        let has_lock: bool = sqlx::query_scalar(
            "SELECT pg_try_advisory_lock($1)"
        )
        .bind(lock_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(false);
        
        if !has_lock {
            // Another instance has the lock, skip this cycle
            continue;
        }
        
        if let Err(e) = evaluate_cycle(&state).await {
            error!("alert engine error: {}", e);
        }
        
        // Lock is released when connection closes
        // For explicit release: SELECT pg_advisory_unlock(42);
    }
}
```

**Priority:** HIGH (for multi-instance deployments; not urgent for single-instance)

---

### 14. ⚠️ HIGH: Alert Errors Treated as "Condition False"

**Location:** [src/alerts/engine.rs#L77-86](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/alerts/engine.rs#L77-L86)

**Issue:**
```rust
let (condition_met, metric_value) = check_condition(...)
    .await
    .unwrap_or((false, None));  // DB error → condition_met = false

if condition_met {
    // Transition to Pending
} else {
    // Can resolve a firing alert due to DB error!
}
```

If the metric query fails (DB down, timeout), the condition is treated as `false`. A **firing** alert will immediately transition to `Ok`, losing its firing state and not retrying.

**Impact:** Transient DB issues cause alerts to auto-resolve incorrectly.

**Fix:**
```rust
match check_condition(...).await {
    Ok((met, val)) => {
        // Process state transition
    }
    Err(e) => {
        error!("alert condition check failed: {}", e);
        // Keep current state; don't transition
        continue;
    }
}
```

**Priority:** HIGH (affects alert reliability)

---

### 15. ⚠️ HIGH: No Graceful Shutdown

**Location:** [src/main.rs](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/main.rs)

**Issue:**
```rust
tokio::spawn(async move {
    alerts::engine::run(alert_state).await;  // Infinite loop, no shutdown signal
});

tokio::spawn(async move {
    retention::run(retention_state).await;  // Infinite loop, no shutdown signal
});

axum::serve(listener, app).await?;  // Server loop
```

On graceful shutdown (SIGTERM), background tasks don't stop cleanly. Partial operations may be in-flight.

**Impact:** Data corruption on restart; incomplete writes.

**Fix:** Use `tokio::select!` and `CancellationToken`:
```rust
use tokio_util::sync::CancellationToken;

let cancel = CancellationToken::new();
let alert_cancel = cancel.clone();
let retention_cancel = cancel.clone();

tokio::spawn(async move {
    tokio::select! {
        _ = alerts::engine::run(alert_state) => {}
        _ = alert_cancel.cancelled() => {
            info!("Alert engine shutting down");
        }
    }
});

// On SIGTERM, signal cancellation
tokio::signal::ctrl_c().await?;
cancel.cancel();
sleep(Duration::from_secs(5)).await;  // Wait for jobs to finish
```

**Priority:** HIGH (for reliability)

---

## Medium-Priority Issues

### 16. ⚠️ MEDIUM: Input Size Not Fully Bounded

**Location:** [src/routes/ingest.rs#L14-L19](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/ingest.rs#L14-L19)

**Issue:**
```rust
if payload.snapshots.is_empty() {
    return Err(StatusCode::BAD_REQUEST);
}
if payload.snapshots.len() > 100 {
    return Err(StatusCode::PAYLOAD_TOO_LARGE);
}
// But: no limits on interfaces[] per snapshot
// No limits on string lengths (hostname, metric names, etc.)
```

An agent could send:
- 100 snapshots
- Each with 1000 interfaces
- Total 100K interface metric inserts per request

**Impact:** Memory exhaustion; slow DB inserts; DoS.

**Fix:**
```rust
const MAX_SNAPSHOTS: usize = 100;
const MAX_INTERFACES_PER_SNAPSHOT: usize = 50;  // Typical systems have <20
const MAX_DISKS_PER_SNAPSHOT: usize = 20;
const MAX_STRING_LEN: usize = 1024;

for snapshot in &payload.snapshots {
    if snapshot.interfaces.len() > MAX_INTERFACES_PER_SNAPSHOT {
        return Err(StatusCode::BAD_REQUEST);
    }
    if let Some(disks) = &snapshot.disk_usage {
        if disks.len() > MAX_DISKS_PER_SNAPSHOT {
            return Err(StatusCode::BAD_REQUEST);
        }
    }
}

// Also add router-layer body size limit
.layer(
    DefaultBodyLimit::max(5_000_000)  // 5 MB max payload
)
```

**Priority:** MEDIUM

---

### 17. ⚠️ MEDIUM: Possible SSRF via Slack Webhook URL

**Location:** [src/alerts/notify.rs#L33-L44](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/alerts/notify.rs#L33-L44)

**Issue:**
```rust
if let Some(ref webhook_url) = slack_webhook {
    match ureq::post(webhook_url)  // User-controlled URL
        .set("Content-Type", "application/json")
        .send_json(payload)
    {
        Ok(_) => info!("slack notification sent"),
        Err(e) => warn!("slack notification failed: {}", e),
    }
}
```

If the webhook URL is not validated, a user could set it to:
- `http://localhost:6379` → Redis
- `http://169.254.169.254/...` → AWS metadata
- Internal IP addresses

**Impact:** SSRF to internal services; credential theft.

**Fix:** Validate URLs at write time (already recommended in issue #11).

**Priority:** MEDIUM (mitigated if Slack URL is validated as `https://hooks.slack.com/*`)

---

### 18. ⚠️ MEDIUM: Unnecessary Blocking I/O in Async Context

**Location:** [src/routes/billing.rs#L137-L148](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/routes/billing.rs#L137-L148)

**Issue:**
```rust
pub async fn get_account(...) -> Result<Json<AccountInfo>, StatusCode> {
    // ...
    let portal_url = if let (Some(ref cust_id), Some(ref key)) = (...) {
        create_portal_session(cust_id, key).ok()  // Blocking HTTP call
    }
}

fn create_portal_session(customer_id: &str, secret_key: &str) -> Result<String, String> {
    let resp = ureq::post("https://api.stripe.com/v1/billing_portal/sessions")  // Blocking
        .set("Authorization", ...)
        .send_form(...)
        .map_err(...)?;
    // ...
}
```

`ureq` is synchronous. Calling it in an async handler blocks the Tokio worker thread. If Stripe is slow, all requests slow down.

**Impact:** Performance degradation; thread pool exhaustion under load.

**Fix:** Move to async HTTP client or separate blocking thread:
```rust
// Option 1: Use reqwest (async)
let client = reqwest::Client::new();
let resp = client.post("https://api.stripe.com/...")
    .send()
    .await?;

// Option 2: Offload to blocking thread
let cust_id = cust_id.to_string();
let key = key.to_string();
let portal_url = tokio::task::spawn_blocking(move || {
    create_portal_session(&cust_id, &key)
})
.await??;

// Option 3: Make portal URL a POST endpoint, not a side effect of GET
```

**Priority:** MEDIUM

---

### 19. ⚠️ MEDIUM: Missing Schema Constraints

**Issue:** No explicit `CHECK` constraints or `ENUM` types for critical fields.

**Examples missing:**
```sql
-- plan should be restricted
ALTER TABLE accounts ADD CONSTRAINT valid_plan 
  CHECK (plan IN ('trial', 'early_access', 'past_due', 'expired'));

-- retention_days should be reasonable
ALTER TABLE accounts ADD CONSTRAINT valid_retention 
  CHECK (retention_days >= 1 AND retention_days <= 730);

-- trial accounts should have trial_ends_at
ALTER TABLE accounts ADD CONSTRAINT trial_has_expiry 
  CHECK (plan != 'trial' OR trial_ends_at IS NOT NULL);

-- api_key prefix should be unique
ALTER TABLE api_keys ADD CONSTRAINT unique_prefix 
  UNIQUE(key_prefix);

-- For deduplication
ALTER TABLE snapshots ADD CONSTRAINT unique_host_time 
  UNIQUE(host_id, time);
```

**Impact:** DB allows invalid states; data inconsistency.

**Fix:** Add migration to apply these constraints.

**Priority:** MEDIUM (low-effort, high-payoff)

---

### 20. ⚠️ MEDIUM: Unwrap/Panic in Token Creation

**Location:** [src/auth.rs#L18-L34](file:///Users/matt/netwatch-cloud/netwatch-cloud/src/auth.rs#L18-L34)

**Issue:**
```rust
pub fn create_token(account_id: Uuid, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::minutes(30))
        .unwrap()  // ← Panics if time math overflows (year 262,144)
        .timestamp() as usize;
    // ...
}
```

Unlikely in practice, but `checked_add_signed` can return `None`.

**Fix:**
```rust
let exp = chrono::Utc::now()
    .checked_add_signed(chrono::Duration::minutes(30))
    .ok_or(jsonwebtoken::errors::Error::from(
        jsonwebtoken::errors::ErrorKind::InvalidAlgorithm
    ))?
    .timestamp() as usize;
```

**Priority:** MEDIUM (low-likelihood but good hygiene)

---

## Summary Table

| ID | Issue | Severity | Effort | Impact | Status |
|---|---|---|---|---|---|
| 1 | Refresh token auth bypass | CRITICAL | S | 28x lifetime extension | Not Fixed |
| 2 | API key panic on short input | CRITICAL | S | DoS | Not Fixed |
| 3 | Cross-tenant host overwrite | CRITICAL | M | Data corruption | Not Fixed |
| 4 | Webhook no idempotency | CRITICAL | M | Billing drift | Not Fixed |
| 5 | Webhook fail-open 200 on error | CRITICAL | S | Billing loss | Not Fixed |
| 6 | Host limit race condition | CRITICAL | M | Usage enforcement | Not Fixed |
| 7 | Ingest partial writes | HIGH | M | Orphaned data | Not Fixed |
| 8 | Ingest deduplication | HIGH | S | Duplicate metrics | Not Fixed |
| 9 | Untrusted timestamps | HIGH | M | Alert unreliability | Not Fixed |
| 10 | Webhook fail-open (no secret) | HIGH | S | Billing compromise | Not Fixed |
| 11 | Slack URL exposed | HIGH | S | Secret leakage | Not Fixed |
| 12 | Alert state lost on restart | HIGH | M | Inconsistent alerts | Not Fixed |
| 13 | Duplicate jobs multi-instance | HIGH | M | Duplicate alerts | Not Fixed |
| 14 | Alert errors reset state | HIGH | S | False resolution | Not Fixed |
| 15 | No graceful shutdown | HIGH | M | Data corruption | Not Fixed |
| 16 | Input size unbounded | MEDIUM | M | DoS/memory | Not Fixed |
| 17 | SSRF via webhook URL | MEDIUM | S | Internal access | Not Fixed |
| 18 | Blocking I/O in async | MEDIUM | M | Performance | Not Fixed |
| 19 | Missing schema constraints | MEDIUM | M | Invalid states | Not Fixed |
| 20 | Unwrap in token creation | MEDIUM | S | Panic (rare) | Not Fixed |

---

## Recommended Fix Priority

### Sprint 1 (This week) — Critical Issues
1. Fix refresh token type checking (1h)
2. Fix API key panic (30m)
3. Fix cross-tenant host overwrite (2h)
4. Fix webhook idempotency + fail-closed (3h)
5. Fix host limit race (2h)
6. Fix webhook fail-open on no secret (30m)

**Total: ~9 hours**

### Sprint 2 (Next week) — High Issues
7. Ingest transactional + deduplication (2h)
8. Timestamp validation (1h)
9. Slack URL masking (1h)
10. Alert error handling (1h)
11. Graceful shutdown (1h)
12. Advisory lock for background jobs (1h)
13. Alert state persistence or logging (2h)

**Total: ~9 hours**

### Sprint 3 (Later) — Medium Issues
14-20: Schema constraints, input bounds, SSRF validation, async cleanup

**Total: ~4 hours**

---

## Deployment Recommendations

**DO NOT** deploy to production until at least Critical issues 1-6 are fixed.

**Suggested rollout:**
1. Fix all Critical + High issues on `main` branch
2. Tag `v0.1.1` (security patch)
3. Deploy to staging, test thoroughly
4. Canary deploy to 10% production traffic
5. Monitor billing/alerts closely for 24h
6. Full production rollout if no issues

---

## Testing Strategy Post-Fix

Add integration tests for:
- Cross-tenant isolation (inject host with foreign account_id, verify rejection)
- Webhook idempotency (post same event_id twice, verify single processing)
- Timestamp validation (send future/past timestamps, verify rejection)
- API key format validation (send invalid lengths, verify no panic)
- Graceful shutdown (send SIGTERM, verify in-flight ops complete)
- Alert error recovery (inject DB error in condition check, verify state persists)

---

## Conclusion

This codebase has solid fundamentals (parameterized queries, reasonable error handling) but needs a focused hardening pass on auth boundaries, data isolation, and error semantics.

The good news: all of these are **fixable in 1-2 focused days**. The suggested approach is **sequential fixes starting with Critical issues**, not a complete rewrite.

After these fixes, the system will be production-grade for small-to-medium SaaS workloads.

---

**Document Status:** Audit Complete  
**Date:** March 31, 2026  
**Next Review:** Post-fix deployment (v0.1.1)
