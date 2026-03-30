# NetWatch Cloud — Roadmap

*Generated 2026-03-30. Based on audit of SPEC-Cloud.md vs current codebase.*

---

## Current State: MVP 95% Complete

**Done:** 30 of 34 build items ✅
**Removed:** 2 (Docker agent, agent.sh) ❌
**Not started:** Stripe billing, design partner testing

---

## Phase 1: Production Hardening (1–2 days)

Priority: Fix issues found during live testing.

| Task | Status | Notes |
|------|--------|-------|
| Fix NUC disk collection (device-based filter) | ✅ Pushed | Agent binary needs rebuild + deploy to NUC |
| Fix NUC ping permissions (CAP_NET_RAW) | ✅ Pushed | `--update` on NUC once API redeploys |
| Fix install.sh POSIX compat (sh/dash) | ✅ Done | Live |
| Fix `/proc` `/sys` access in systemd unit | ✅ Done | Live |
| Fix `netwatch.dev` references → Railway URLs | ✅ Done | Live |
| Fix Resend from address | ✅ Done | Using `onboarding@resend.dev` |
| Tag agent release for `--update` to work | ❌ TODO | Need `v0.1.0` tag on netwatch-cloud for GitHub Releases |
| Cross-compile agent + publish GitHub Release | ❌ TODO | Releases CI exists but no tag yet |

## Phase 2: Missing Spec Features (3–5 days)

Features described in the spec but not yet implemented.

| Task | Spec Section | Effort | Impact |
|------|-------------|--------|--------|
| **Metrics downsampling** | §3.4 | 2h | Critical for 72h views — without it, 17K+ data points per chart |
| **Host DELETE endpoint** | §3.2 | 30m | Users can't remove decommissioned hosts |
| **Account GET/PUT endpoints** | §3.2 | 1h | Can't change notification prefs (email on/off, Slack webhook) |
| **Refresh tokens** | §3.2 | 2h | Currently JWT expires and user is logged out with no refresh |
| **Alert notification rate limiting** | §6.5 | 1h | Can spam Slack/email if alert keeps firing (1/15min per rule, 50/hr cap) |
| **207 Multi-Status ingest** | §3.3 | 1h | Partial acceptance if some snapshots in batch are invalid |
| **cargo audit + npm audit in CI** | §8.1 | 30m | Security scanning |
| **HSTS headers** | §8.1 | 15m | |
| **CSP headers** | §8.1 | 15m | |

## Phase 3: Stripe Billing (3–5 days)

The only major unstarted build item.

| Task | Details |
|------|---------|
| Add `stripe` crate to Rust deps | |
| Create Stripe Product + Price ($49/mo) | Manual in Stripe dashboard |
| Create Stripe Customer on registration | Set `trial_ends_at = now + 14d` |
| Add Stripe webhook endpoint | Handle `subscription.updated`, `subscription.deleted`, `invoice.payment_failed` |
| Enforce trial limits | Ingest returns 402 after trial expires without payment |
| Enforce host limits | Trial: 3 hosts, Early Access: 10 hosts |
| Enforce retention limits | Trial: 24h, Early Access: 72h |
| Settings page: billing section | Link to Stripe Customer Portal, trial countdown |
| Dashboard banner | "Trial expires in X days" / "Trial expired — add payment method" |

## Phase 4: Agent Release Pipeline (1 day)

| Task | Details |
|------|---------|
| Tag `v0.1.0` on netwatch-cloud | Triggers release.yml → builds agent binaries |
| Verify `--update` flow end-to-end | Agent downloads from GitHub Releases, replaces itself |
| Update install.sh download URL | Currently points to `latest` release |
| Add agent version to fleet dashboard | Show which hosts have outdated agents |

## Phase 5: Distribution & Launch (2–3 days)

| Task | Details |
|------|---------|
| Buy domain (`netwatch.run` recommended) | $15/yr |
| Configure DNS → Railway custom domains | `api.netwatch.run`, `app.netwatch.run` |
| Update all hardcoded URLs | Agent default, install script, web, emails |
| README with screenshots | |
| Write Show HN post | |
| Submit to Awesome lists | awesome-rust, awesome-selfhosted, awesome-sysadmin |
| Reddit posts | r/selfhosted, r/sysadmin, r/homelab |
| LinkedIn announcement | |

## Phase 6: Post-Launch Polish (ongoing)

| Task | Priority | Notes |
|------|----------|-------|
| Design partner testing (3–5 users) | High | Get real feedback before scaling |
| Integration tests | Medium | No test files exist currently |
| Sentry error tracking | Low | "Add later if needed" |
| S3 backup automation | Low | Currently Railway auto-backup only |
| Password reset flow | Medium | Not in spec but users will need it |
| Email verification | Low | Currently no verification on signup |

---

## Explicitly Out of Scope (from spec §12)

These are **not planned** and should stay that way for now:

- macOS/Windows agent, Packet capture, WebSockets, GraphQL
- SSO/OAuth, Multi-user/teams, Shared dashboards
- Self-hosted option, Mobile app, Custom themes
- Prometheus/Grafana export, Terraform provider, Helm chart
- TimescaleDB, Redis, Protobuf

---

## Key Metrics Gap: What Works vs What Doesn't

| Metric | Mac Agent | Linux Agent (NUC) |
|--------|-----------|-------------------|
| CPU % | ✅ | ✅ |
| CPU per core | ❌ (not implemented) | ✅ (but all zeros when idle) |
| Memory | ✅ | ✅ |
| Load avg | ❌ (macOS uses ps, inaccurate) | ✅ |
| Disk usage | ✅ | ❌ Binary needs rebuild |
| Disk I/O | ❌ (not on macOS) | ✅ |
| Swap | ✅ | ✅ |
| Connections | ✅ | ✅ |
| Gateway latency | ✅ | ✅ (after CAP_NET_RAW fix) |
| Packet loss | ✅ | ✅ (after CAP_NET_RAW fix) |
| Network RX/TX | ✅ | ✅ |
| TCP states | ✅ | ✅ |
