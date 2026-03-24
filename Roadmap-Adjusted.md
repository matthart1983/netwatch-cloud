# NetWatch — Adjusted SaaS Plan (Post-Critique)

This document is the result of a radical critique of the original business plan. It strips away the fantasy and focuses on what a solo founder should actually build and validate.

---

## Part 1: How The Original Plan Fails

### The Core Problem: This Is 5 Companies Disguised As One

The original roadmap tries to build a mini-Datadog, mini-CloudShark, mini-PagerDuty, mini-Grafana, an auth/billing platform, and an enterprise product — as a solo founder. That's not ambitious. It's self-sabotage.

### Fatal Flaw #1: The "Gap" Doesn't Exist Like We Think

The claim "there is no lightweight, developer-friendly network monitoring tool that bridges the gap" is mostly false. Teams already solve this with:

- Prometheus + Grafana + node_exporter (free, mature, trusted)
- Netdata (free, real-time, web UI, already does what we'd build)
- Better Stack, Zabbix, Checkmk (established, cheap)
- Cloud-provider networking dashboards and flow logs
- Plain old `tcpdump`, `ss`, `mtr` during incidents

Network observability is often **not a standalone budget line**. Buyers want it integrated into their existing stack, not as a new tool.

### Fatal Flaw #2: Open-Source Users Are Not SaaS Buyers

The plan assumes GitHub stars → downloads → paid subscribers. This is the classic open-source monetization trap:

- OSS users like free tools
- OSS users like self-hosting
- OSS users are privacy-sensitive about network telemetry
- OSS users enjoy tinkering more than paying
- The TUI is already feature-rich enough that the paid upsell (history, alerts, fleet view) feels optional

**A star is not a dollar. Attention is not revenue.**

### Fatal Flaw #3: The Audience Is Too Broad

The plan targets homelabbers, small DevOps teams, mid-market SRE, MSPs, and enterprise simultaneously. Each has different budgets, risk tolerance, procurement processes, security needs, and support expectations. Building for all of them means building for none of them.

### Fatal Flaw #4: Network Debugging Is Episodic, Not Continuous

Most network debugging is incident-driven and ad hoc. Teams reach for tools during a crisis, then forget about them. This means:

- Strong praise during incidents, weak retention afterward
- The emotional moment is "this saved me Tuesday"
- The billing feeling is "do I need this every month?"
- **Churn will be the killer**

### Fatal Flaw #5: The Pricing Is Wrong In Both Directions

**Too low for serious teams:** $79/mo for unlimited users + SSO + shared dashboards + custom alert rules is absurdly underpriced. SSO alone signals higher ACV.

**Too high for hobbyists:** $9–29/mo attracts curious tinkerers who churn fast and create support load.

The plan risks attracting cheap, privacy-sensitive, high-support, low-retention customers — the worst possible combination.

### Fatal Flaw #6: The Revenue Projections Are Fiction

- 1–2% free-to-paid conversion is not "conservative" — it's optimistic for infra tools
- 50,000 free users by month 24 assumes viral growth that network tools rarely achieve
- "Break-even at 11 Starter customers" ignores your salary, taxes, legal, support time, and the opportunity cost of not having a job

### Fatal Flaw #7: Massive Over-Engineering

The architecture is too clever too early:

| Proposed | Problem |
|----------|---------|
| TUI as agent (`--agent` flag) | TUI and daemon have different concerns — lifecycle, privileges, reliability. Build a separate agent |
| WebSocket agent protocol | Harder to operate, retry, load-balance. Use HTTPS batch POST |
| Protobuf/MessagePack schemas | Premature optimization before you have customers. Use JSON |
| Redis from day one | Postgres alone is enough for a long time |
| TimescaleDB with generic metric table | Flexible but miserable for alert queries and debugging |
| Browser-based PCAP analysis | This isn't a feature — it's an entire product. CloudShark spent years on this |
| AI insights in cloud | Hurts credibility early. Serious operators won't trust it until basics are solid |
| Self-hosted option | Focus suicide for a solo founder. Cannibalizes hosted product |
| Cross-platform agent (macOS/Linux/Windows) | Hidden tax — different privileges, capture stacks, service management |

### Fatal Flaw #8: PCAP Storage Is A Legal Nightmare

Packet captures contain credentials, cookies, internal hostnames, regulated data. Storing this in multi-tenant SaaS creates massive privacy/compliance burden and customer fear. A single breach kills the company.

### Fatal Flaw #9: The GTM Plan Is Awareness Theater

GitHub stars, Product Hunt, RustConf talks, HN spikes, YouTube demos — these feel productive but don't move infra sales. Real GTM for infra SaaS is founder-led sales, design partners, direct outreach, and trust building.

### Fatal Flaw #10: The Chicken-and-Egg Agent Problem

Value depends on data → data depends on installation → installation requires root/sudo + trust → trust requires demonstrated value → demonstrated value requires data. This loop is brutal for new products.

---

## Part 2: The Adjusted Plan — What To Actually Build

### New Product Thesis

Stop positioning as "network monitoring platform." Position as:

> **Historical network debugging for small Linux fleets** — for teams that occasionally get burned by packet loss, DNS weirdness, interface flaps, and transient connection issues but don't want Datadog complexity.

### Pick One Customer

**Small infra/DevOps teams managing 5–50 Linux servers.** Not homelabbers. Not enterprise. Not MSPs. One customer, one pain point.

### The Real MVP (4–8 Weeks)

Build only this:

1. **Separate lightweight Linux agent daemon** (not `--agent` on the TUI)
   - systemd service, not a TUI fork
   - No packet capture at first
   - No Windows/macOS
   - Runs without root for basic metrics

2. **Simple HTTPS batch ingest**
   - POST JSON every 15–30s
   - No WebSocket
   - No remote command channel
   - No protobuf

3. **Only 5 metrics**
   - Interface up/down events
   - Packet loss to gateway/DNS
   - Gateway/DNS latency
   - Connection count
   - Host heartbeat (online/offline)

4. **Tiny web app**
   - Host list with status indicators
   - Single host detail page with 24–72h history
   - Email + Slack alerts for the 5 metrics above
   - Account/API key management
   - That's it. Nothing else.

5. **No enterprise anything**
   - No SSO, no RBAC, no shared dashboards
   - No PCAP browser, no AI, no GraphQL, no Terraform
   - No self-hosted option

### New Pricing

Don't start at $9. Start high and validate:

| Tier | Price | What You Get |
|------|-------|-------------|
| **Early Access** | $49–$99/mo flat | Up to 10 hosts, 30-day history, email + Slack alerts, direct founder support |

That's it. One tier. Manual onboarding. You personally help each customer install.

### Validation Gates (Before Building More)

Do not build beyond the MVP until:

- [ ] 20 customer interviews completed
- [ ] 5 design partners actively using the agent
- [ ] 3 paying customers ($49+/mo)
- [ ] Customers use it after their first incident (not just during)
- [ ] At least 2 customers renew after 60 days
- [ ] You know which single feature they'd pay more for

### What NOT To Build Until Proven

| Feature | Gate |
|---------|------|
| Team/collaboration features | When one org has >2 active users asking for it |
| PagerDuty/webhook integrations | When >5 customers ask for the same one |
| Packet capture upload | When customers say metric summaries aren't enough |
| AI insights | When core alerting is trusted and retained |
| Self-hosted | When prospects are blocked solely by hosting (not price) |
| Enterprise auth/compliance | When real enterprise demand appears with budget |
| Cross-platform agent | When Linux is stable and generating revenue |

### The Real Metrics That Matter

Ignore:
- GitHub stars
- Downloads
- NPS scores
- Product Hunt rank

Track:
- % who install agent successfully
- Time from signup to first data appearing
- % who connect 3+ hosts
- % who configure at least one alert
- Weekly active users investigating incidents
- 30/60-day paid retention
- Support tickets per customer per month

### Honest Timeline

```
Week  1–2   ████ Customer interviews (20 conversations)
Week  3–6   ████████ Build Linux agent daemon + HTTPS ingest + minimal web app
Week  7–8   ████ Onboard 5 design partners manually
Week  9–12  ████████ Iterate based on real usage, launch $49/mo tier
Month 4–6   ████████████ Grind to 10 paying customers or pivot
```

---

## Part 3: The Hard Truth

The original roadmap is probably a failure as written. Not because it can't be built — it probably can. But because it builds **too much, for too many audiences, at too low a price, before proving that:**

1. People will install it
2. People will trust it with network data
3. People will use it continuously (not just during incidents)
4. People will renew

**Keep the TUI excellent and free. Build a separate, much smaller hosted product. Validate one painful workflow. Charge more, sooner. Ignore stars. Don't try to become Datadog, CloudShark, Grafana, and PagerDuty at once.**
