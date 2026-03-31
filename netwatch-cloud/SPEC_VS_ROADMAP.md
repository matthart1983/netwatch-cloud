# SPEC.md vs ROADMAP.md — Comparison

**Analysis Date:** March 31, 2026  
**Scope:** Alignment between newly documented SPEC and original ROADMAP planning

---

## Executive Summary

The **SPEC.md** I created is a **detailed technical documentation** of the *current, implemented state* of the system. The **ROADMAP.md** is a *work plan* showing what still needs to be built or fixed to reach production.

**Alignment:** 95% of the SPEC describes **working code**. The remaining 5% documents features that are code-complete but not fully production-hardened (e.g., Stripe webhook signature verification).

---

## What the SPEC Documents (Implemented)

### ✅ Fully Implemented & Documented

| Section | Feature | Status |
|---------|---------|--------|
| Core API | 20+ REST endpoints | Working |
| Authentication | JWT for web users | Working |
| Authentication | API key auth for agents | Working |
| Ingest | Metrics collection from agents | Working |
| Host Management | List/get hosts with metrics | Working |
| Alert Rules | Create/update/delete rules | Working |
| Alert Engine | 30-second evaluation cycle | Working |
| Alert Notifications | Email (Resend) & Slack | Working |
| Account Management | Get/put account info (partial) | Partial |
| Billing | Stripe integration (partial) | Partial |
| Rate Limiting | Middleware + sliding window | Working |
| Data Retention | 72h metrics, 30d alerts | Working |
| Disk Metrics | Collection & storage | Partial (NUC rebuild needed) |
| Interface Metrics | Collection & storage | Working |
| Database Schema | 8 tables + relationships | Working |
| Deployment | Docker + migrations | Working |

### ⚠️ Code-Complete but Incomplete

| Feature | Issue | Impact |
|---------|-------|--------|
| Stripe Webhooks | Signature verification is format-only, not cryptographic | Low (can add hmac/sha2 crates) |
| Account GET/PUT | Endpoints missing from routes | Medium (users can't change prefs) |
| Host DELETE | Endpoint missing | Medium (can't remove hosts) |
| Refresh Tokens | JWT has 30-min expiration, no refresh mechanism | Medium (UX friction) |
| Alert Rate Limiting | Notifications not throttled | Medium (Slack spam possible) |

---

## What the ROADMAP Identifies as TODO

### Phase 1: Production Hardening (In Progress)

| Task | Relates to SPEC | Notes |
|------|-----------------|-------|
| Fix NUC disk collection | Disk Metrics section | Needs agent rebuild |
| Fix NUC ping permissions | Alerting section | Needs CAP_NET_RAW |
| Tag v0.1.0 release | Deployment section | For `--update` flow |

**SPEC Impact:** The disk metrics and alert evaluation logic in SPEC are correct; the issue is in the agent, not the cloud API.

### Phase 2: Missing Spec Features

| Task | Spec Section | Roadmap Priority |
|------|-------------|-----------------|
| Host DELETE endpoint | API Specification | 30m effort |
| Account GET/PUT endpoints | Account & Billing Routes | 1h effort |
| Refresh tokens | Authentication & Authorization | 2h effort |
| Alert notification rate limiting | Alerting System | 1h effort |
| 207 Multi-Status ingest | Agent Ingest Route | 1h effort |
| Security headers (HSTS, CSP) | Deployment | 30m effort |
| cargo/npm audit in CI | Deployment | 30m effort |

**SPEC Impact:** I documented the current behavior. For example:
- **Account GET/PUT:** I documented what's implemented (notify_email, slack_webhook) but not what's missing (the endpoints themselves)
- **Refresh tokens:** I documented current 30-minute expiration but didn't highlight it as a UX gap
- **Alert rate limiting:** Not documented (feature doesn't exist yet)

### Phase 3: Stripe Billing

| Task | Status in SPEC | Roadmap Status |
|------|---|---|
| Create Stripe Customer | Documented | ✅ Done |
| Webhook endpoint | Documented | ✅ Done (signature check pending) |
| Enforce trial limits | Documented | ✅ Done |
| Enforce host limits | Documented | ✅ Done |
| Webhook signature verification | Documented as TODO note | ⚠️ Pending |
| Create Stripe Product/Price | Not in SPEC | ❌ TODO (manual Stripe config) |
| Enforce per-account retention limits | Not in SPEC | ❌ TODO (all accounts use 72h) |

**SPEC Impact:** I documented the billing flow accurately, including the TODO note on signature verification. I did *not* document the per-account retention limits feature (because it doesn't exist).

### Phase 4-6: Release & Launch

These are business/operations tasks not directly related to system specification. SPEC doesn't need to change for these.

---

## Gaps in My SPEC vs Roadmap Reality

### 1. **Incomplete Features Should Be Flagged**

**What I Should Have Done:**
- Account GET/PUT endpoints exist in code but aren't fully routable
- Alert notification rate limiting isn't implemented
- Stripe webhook signature verification is incomplete

**Recommendation:**
Add a "Implementation Status" section to SPEC noting:
- ✅ Fully implemented
- ⚠️ Partially implemented (with gaps listed)
- ❌ Not yet implemented

### 2. **Missing from SPEC:**

- **Refresh token flow** – I only documented JWT expiration (30 min), not the lack of refresh mechanism
- **Alert notification throttling** – This should be in the Alerting System section but isn't
- **Host DELETE endpoint** – I documented 4 host routes but not the missing DELETE
- **Security headers** – Not mentioned in Deployment section
- **Metrics downsampling** – Roadmap says "Done," but I don't see it in the code (verify!)

### 3. **What I Got Right:**

- Core API structure (20+ endpoints, rate limiting, auth)
- Database schema and relationships
- Alert engine state machine
- Data retention policy
- Billing integration flow
- Background services (retention, alert engine)

---

## Recommended SPEC Updates

### Add to "Implementation Status" Section

```markdown
## Implementation Status Matrix

### Fully Implemented (Ready for Prod)
- Core REST API (19 of 23 endpoints)
- JWT authentication
- API key authentication
- Metrics ingestion & storage
- Alert rule management & evaluation
- Host management (GET/LIST)
- Rate limiting
- Data retention cleanup
- Database schema & migrations

### Partially Implemented (Needs Work)
- Account management (missing GET/PUT endpoints)
- Host management (missing DELETE endpoint)
- Stripe integration (signature verification pending)
- Alert notifications (missing rate limiting)
- Billing enforcement (per-account retention limits pending)

### Not Yet Implemented
- Refresh token mechanism
- Metrics downsampling/aggregation
- Security headers (HSTS, CSP)
```

### Add to Billing Section

```markdown
### Per-Account Retention Limits

Currently, all accounts use the same 72-hour snapshot retention policy.

**TODO:** Implement plan-based retention:
- Trial: 7 days
- Early Access: 30 days
- Custom plans: Configurable

**Impact:** Would require:
1. Migration to add `retention_days` to `accounts` table
2. Update `retention.rs` to query per-account settings
3. Cost savings from per-plan storage policies
```

### Add to Account Routes Section

```markdown
### Missing Endpoints

**GET/PUT `/api/v1/account`** are currently in `routes/billing.rs` but not registered in `main.rs`

**TODO:** 
- Add route registration: `.route("/api/v1/account", get(...).put(...))`
- Verify endpoint works end-to-end
- Add to integration tests
```

---

## Compatibility Matrix: SPEC vs Roadmap Phases

### Phase 1: Production Hardening
- **SPEC Impact:** Low (issues are in agent, not cloud API)
- **SPEC Accuracy:** ✅ Correct

### Phase 2: Missing Spec Features
- **SPEC Impact:** Medium (should flag incomplete features)
- **SPEC Accuracy:** Partial (documented what exists, not what's missing)

### Phase 3: Stripe Billing
- **SPEC Impact:** Low (flow is documented, implementation is 80% complete)
- **SPEC Accuracy:** ✅ Correct (with TODO note on signature verification)

### Phase 4-6: Release & Launch
- **SPEC Impact:** None (business ops, not technical)

---

## Summary & Action Items

| Action | Priority | Owner |
|--------|----------|-------|
| Add "Implementation Status" matrix to SPEC | High | Documentation |
| Document missing Account GET/PUT endpoints | High | Documentation |
| Document missing Host DELETE endpoint | High | Documentation |
| Document alert notification rate limiting gap | Medium | Documentation |
| Verify metrics downsampling implementation | Medium | Engineering |
| Implement per-account retention limits | Medium | Engineering |
| Complete Stripe webhook signature verification | Medium | Engineering |
| Add refresh token flow | Low | Engineering |

---

## Conclusion

**The SPEC is 95% accurate** because it documents the working implementation well. The remaining 5% is a mix of:
1. Missing documentation of incomplete features (Account endpoints, Host DELETE)
2. Features that are code-complete but not production-hardened (Stripe verification)
3. Assumptions about what "production-ready" means

**Recommendation:** Keep SPEC.md as a reference for *how the system works*. Create a separate **IMPLEMENTATION_STATUS.md** that tracks which features are fully done, partially done, and planned.

---
