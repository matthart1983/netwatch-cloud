# Host Dashboard V2 — Phase 2: Dockable Chart Viewer

## Status: ✅ Implemented

Both the **Host Detail** dashboard and the **Fleet Metrics** section now have dockable panel systems with drag-reorder, collapse, maximize, lock, and reset — with layout persistence via localStorage.

---

## Implementation Summary

### Approach: Native HTML Drag-and-Drop

The spec originally proposed `react-grid-layout` for panel management. The actual implementation uses **native HTML5 drag-and-drop** (`draggable`, `onDragStart`, `onDragOver`, `onDrop`) for panel reordering within a standard CSS grid. This avoids the `react-grid-layout` dependency for panel positioning while keeping the library installed for potential future use.

**Trade-offs vs react-grid-layout:**
- ✅ Simpler — no RGL configuration, responsive breakpoints, or CSS imports needed
- ✅ Lighter — no additional resize observers or layout calculations
- ❌ No free-form resize (panels have fixed heights, not user-resizable)
- ❌ No arbitrary grid placement (reorder only, not 2D positioning)

---

## Host Detail Dashboard (`web/src/app/hosts/[id]/page.tsx`)

~730 lines. Single-file architecture (Phase 2A approach).

### Panels (7 total)

| Panel ID | Title | Data Keys |
|---|---|---|
| `latency-loss` | Latency & Loss | gateway_rtt, dns_rtt, loss |
| `network-conn` | Network & Connections | net_rx, net_tx, connections |
| `cpu-memory` | CPU & Memory | cpu, mem_used, mem_avail |
| `cpu-per-core` | CPU per Core | core_0..core_N (dynamic), cpu total |
| `load-swap` | Load & Swap | load_1m/5m/15m, swap_used |
| `disk-util` | Disk Utilisation | disk_usage |
| `tcp-states` | TCP Connection States | time_wait, close_wait |

**Note:** `cpu-per-core` was added post-spec. It spans full width (`lg:col-span-2`), has a taller default height (360px vs 280px), and dynamically generates line configs at render time by scanning data for `core_*` keys.

### State Management

```typescript
const LS_KEY = 'host-dashboard-state-v4'

interface DashState {
  collapsed: Record<string, boolean>  // which panels are collapsed
  order: string[]                     // panel ordering
}
```

**Divergence from spec:** The spec proposed persisting full `ResponsiveLayouts` (positions/sizes per breakpoint). The implementation only persists **order** and **collapsed** state, since panels can't be freely positioned or resized.

### Panel Chrome (`ChartPanel` component)

```
┌─────────────────────────────────────────────────┐
│ ⠿ Latency & Loss   now 12.3  avg 11.2  max 15  ⤢  ∧  │
├─────────────────────────────────────────────────┤
│                                                 │
│          [Recharts LineChart]                    │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Header controls (left to right):**
- `GripVertical` — drag handle (hidden when locked)
- Title text
- Summary stats: now / avg / max / min (hidden when collapsed, hidden on small screens)
- `Maximize2` — full-screen overlay
- `ChevronUp`/`ChevronDown` — collapse toggle

### Toolbar

Lock/Unlock and Reset buttons sit in the time-range row:

```
[1h] [6h] [24h] [72h]  [Live indicator]    [🔓 Unlocked] [↺ Reset]
```

- **Lock:** Hides grip handles, sets `draggable={false}`. Visual state: emerald badge.
- **Reset:** Clears localStorage, resets order and collapsed state. No confirmation dialog (diverges from spec).

### Maximized Overlay (`MaximizedOverlay` component)

- `fixed inset-0 z-30 bg-zinc-950/98`
- Full-viewport overlay (covers entire page, not just chart area — diverges from spec)
- Header bar with title, summary stats, `Minimize2` close button
- Escape key listener to close
- `syncId="host-dashboard"` maintained for tooltip crosshair sync
- Click on backdrop also closes

### Drag Behaviour

```typescript
draggable={!locked}
onDragStart={() => handleDragStart(config.id)}
onDragOver={(e) => handleDragOver(e, config.id)}
onDrop={() => handleDrop(config.id)}
onDragEnd={handleDragEnd}
```

- Drop target highlighted with `ring-2 ring-emerald-500/50`
- Panel order saved to localStorage on every drop

---

## Fleet Metrics Dashboard (`web/src/app/page.tsx`)

Added in the same commit. Applies identical docking patterns to the Fleet Metrics section on the hosts overview page.

### Panels (8 total)

| Index | Title | Multi-host overlay |
|---|---|---|
| 0 | Gateway Latency (ms) | One line per host |
| 1 | Packet Loss (%) | One line per host |
| 2 | Network I/O (KB) | RX + TX per host (TX dashed) |
| 3 | CPU Usage (%) | One line per host |
| 4 | Memory Usage (%) | One line per host |
| 5 | Load Average (1m) | One line per host |
| 6 | Swap Used (MB) | One line per host |
| 7 | Disk Usage (%) | One line per host |
| 8 | Connections | One line per host |

### State Management

```typescript
const FLEET_LS_KEY = 'fleet-dashboard-state-v1'

interface FleetDashState {
  collapsed: Record<string, boolean>
  order: string[]   // panel indices as strings ("0", "1", ...)
}
```

### Components

- **`FleetChartPanel`** — Panel chrome with grip handle, title, maximize/collapse buttons. Uses `syncId="fleet-dashboard"`.
- **`FleetMaximizedOverlay`** — Full-screen overlay with host color legend in header. Escape key to close.
- **`buildFleetChartData()`** — Extracted helper that builds `{ data, lines }` for a given chart config, memoized in each panel.

### Toolbar

```
Fleet Metrics                              [🔓 Unlocked] [↺ Reset]
```

Same lock/reset pattern as host dashboard.

---

## CSS

`web/src/app/globals.css` contains dark-theme overrides for `react-grid-layout` (placeholder + resize handle styling), added during initial RGL exploration. These remain but are currently unused since the implementation uses native drag-and-drop.

---

## Spec vs Implementation Divergences

| Spec Item | Status | Notes |
|---|---|---|
| react-grid-layout for positioning | ❌ Not used | Native HTML drag-and-drop instead. RGL still in `package.json`. |
| Panel resize (drag bottom-right) | ❌ Not implemented | Panels have fixed heights (240px fleet, 280/360px host). |
| 12-column grid system | ❌ Not used | Standard CSS grid `grid-cols-1 lg:grid-cols-2`. |
| Responsive breakpoints (lg/md/sm/xs) | ✅ Partial | CSS grid handles responsive via `lg:grid-cols-2`, single column on mobile. |
| Panel drag reorder | ✅ Implemented | Native drag-and-drop with visual drop indicator. |
| Panel collapse | ✅ Implemented | Header-only mode, height set to `auto`. |
| Panel maximize | ✅ Implemented | Full-viewport overlay with Escape key. |
| Layout lock | ✅ Implemented | Hides handles, disables draggable. |
| Layout reset | ✅ Implemented | Clears localStorage, no confirm dialog. |
| Layout persistence | ✅ Implemented | Order + collapsed state in localStorage. |
| Summary stats in panel header | ✅ Host only | Fleet panels don't show stats (no meaningful single-value summary for multi-host). |
| `syncId` crosshair | ✅ Implemented | `"host-dashboard"` for host, `"fleet-dashboard"` for fleet. |
| Phase 2B component extraction | ❌ Not done | Both dashboards remain single-file. |
| `cpu-per-core` panel | ✅ Added | Not in original spec. Full-width, dynamic core detection. |
| Fleet Metrics docking | ✅ Added | Not in original spec. Same pattern as host dashboard. |
| Remove CollapsibleSection | ✅ Done | Replaced by panel chrome system. |
| `host-dashboard-sections` cleanup | ✅ Done | Old localStorage key no longer used. |

---

## Acceptance Criteria (Updated)

- [x] All 7 host chart panels render in dockable grid
- [x] All 8 fleet chart panels render in dockable grid
- [x] Panels can be dragged by handle to reorder
- [x] Maximize button opens full-viewport overlay; Escape/button closes
- [x] Collapse button shrinks panel to header-only; expand restores
- [x] Layout (order + collapsed) persists to localStorage per dashboard
- [x] Lock toggle disables drag; Reset restores defaults
- [x] Responsive: 2-col on desktop, 1-col on mobile
- [x] All Phase 1 features remain functional
- [x] TypeScript compiles cleanly (`tsc --noEmit`)
- [x] Dark theme consistent
- [ ] Panel resize via drag (not implemented — fixed heights)
