# Host Dashboard V2 — Phase 2: Dockable Chart Viewer

## Overview

Replace the current static CSS grid chart layout with a fully interactive panel system where chart panels are **draggable, resizable, maximizable, and dockable**. Users can customise their dashboard layout and have it persisted across sessions.

**Library:** `react-grid-layout` v2.2.3 (already installed, React 18+ compatible)

---

## Current State (Phase 1)

The dashboard (`web/src/app/hosts/[id]/page.tsx`, ~630 lines) currently has:

- **Fixed elements** (keep as-is): Sticky health header, host info panel, live stats bar, time range selector, live/pause indicator
- **Chart area** (replace): 3 `CollapsibleSection` containers (Network, System, Storage), each with a 2-col CSS grid holding `DashboardChart` cards. 6 chart panels total:

| Panel ID | Title | Section | Lines/Data Keys |
|---|---|---|---|
| `latency-loss` | Latency & Loss | Network | gateway_rtt, dns_rtt, loss |
| `network-conn` | Network & Connections | Network | net_rx, net_tx, connections |
| `cpu-memory` | CPU & Memory | System | cpu, mem_used, mem_avail |
| `load-swap` | Load & Swap | System | load_1m/5m/15m, swap_used |
| `disk-util` | Disk Utilisation | Storage | disk_usage |
| `tcp-states` | TCP Connection States | Storage | time_wait, close_wait |

---

## Architecture

### What Changes

```
BEFORE (Phase 1):                    AFTER (Phase 2):
┌─────────────────────────┐          ┌─────────────────────────┐
│ Sticky Health Header    │          │ Sticky Health Header    │  ← unchanged
│ Host Info (collapsible) │          │ Host Info (collapsible) │  ← unchanged
│ Live Stats Bar          │          │ Live Stats Bar          │  ← unchanged
│ Time Range Selector     │          │ Time Range + Lock/Reset │  ← add toolbar
├─────────────────────────┤          ├─────────────────────────┤
│ ▸ Network Health        │          │ ┌──────────┬──────────┐ │
│   ┌───────┬───────┐     │          │ │ Panel    │ Panel    │ │
│   │ chart │ chart │     │          │ │ (drag   )│ (drag   )│ │  ← react-grid-layout
│   └───────┴───────┘     │          │ ├──────────┼──────────┤ │
│ ▸ System Resources      │          │ │ Panel    │ Panel    │ │
│   ┌───────┬───────┐     │          │ │          │          │ │
│   │ chart │ chart │     │          │ ├──────────┼──────────┤ │
│   └───────┴───────┘     │          │ │ Panel    │ Panel    │ │
│ ▸ Storage               │          │ └──────────┴──────────┘ │
│   ┌───────┬───────┐     │          └─────────────────────────┘
│   │ chart │ chart │     │
│   └───────┴───────┘     │
└─────────────────────────┘
```

### What Stays the Same

- All Phase 1 features: health header, live stats, host info, live/pause, time range
- All chart internals: Recharts config, data transformation, deduplication, colors, `syncId="host-dashboard"`
- `DashboardChart` component (summary stats in header)
- Tooltip styling, Line props (`connectNulls`, `interval="preserveStartEnd"`, no `type` prop)

---

## Detailed Spec

### 1. Panel System via react-grid-layout

**Grid Configuration:**
- 12-column grid (standard)
- `rowHeight`: 120px (each chart panel = 2h = 240px content area)
- Responsive breakpoints: `{ lg: 1024, md: 768, sm: 480, xs: 0 }`
- Columns per breakpoint: `{ lg: 12, md: 12, sm: 6, xs: 6 }`
- Width via `useContainerWidth` hook (v2 API)

**Default Layout (6 panels, 2-col on lg/md):**

```typescript
const DEFAULT_LAYOUTS: ResponsiveLayouts = {
  lg: [
    { i: 'latency-loss',  x: 0, y: 0, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'network-conn',  x: 6, y: 0, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'cpu-memory',    x: 0, y: 2, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'load-swap',     x: 6, y: 2, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'disk-util',     x: 0, y: 4, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'tcp-states',    x: 6, y: 4, w: 6, h: 2, minW: 4, minH: 2 },
  ],
  md: [
    { i: 'latency-loss',  x: 0, y: 0, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'network-conn',  x: 6, y: 0, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'cpu-memory',    x: 0, y: 2, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'load-swap',     x: 6, y: 2, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'disk-util',     x: 0, y: 4, w: 6, h: 2, minW: 4, minH: 2 },
    { i: 'tcp-states',    x: 6, y: 4, w: 6, h: 2, minW: 4, minH: 2 },
  ],
  sm: [
    { i: 'latency-loss',  x: 0, y: 0,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'network-conn',  x: 0, y: 2,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'cpu-memory',    x: 0, y: 4,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'load-swap',     x: 0, y: 6,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'disk-util',     x: 0, y: 8,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'tcp-states',    x: 0, y: 10, w: 6, h: 2, minW: 6, minH: 2 },
  ],
  xs: [
    { i: 'latency-loss',  x: 0, y: 0,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'network-conn',  x: 0, y: 2,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'cpu-memory',    x: 0, y: 4,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'load-swap',     x: 0, y: 6,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'disk-util',     x: 0, y: 8,  w: 6, h: 2, minW: 6, minH: 2 },
    { i: 'tcp-states',    x: 0, y: 10, w: 6, h: 2, minW: 6, minH: 2 },
  ],
}
```

### 2. Panel Features

Each chart panel (`DashboardChart`) gets a **panel chrome** — a header bar with controls:

```
┌─────────────────────────────────────────────────┐
│ ⠿ Latency & Loss    now 12.3  avg 11.2  ⤢  ✕  │  ← drag handle + title + stats + maximize + collapse
├─────────────────────────────────────────────────┤
│                                                 │
│          [Recharts LineChart]                    │
│                                                 │
└─────────────────────────────────────────────────┘
```

**Panel Header Controls (left to right):**
- **Drag handle** (`⠿` / `GripVertical` icon from lucide-react) — only this area initiates drag
- **Title** — chart name + summary stats (current/avg/max/min) as today
- **Maximize button** (`Maximize2` icon) — expands panel to full viewport overlay
- **Collapse button** (`ChevronUp` → `ChevronDown`) — minimise panel to header-only (h=1)

**Panel Capabilities:**

| Feature | Behaviour |
|---|---|
| **Drag/Move** | Drag by handle to reorder. Other panels reflow automatically (vertical compaction). |
| **Resize** | Drag bottom-right corner. Chart `ResponsiveContainer` adapts automatically. Min 4-col wide, 2-row tall. |
| **Maximize** | Fixed-position overlay covering the chart area (not full page — header/stats stay visible). Shows chart at full width/height. Click maximize again or press Escape to restore. Only one panel maximized at a time. |
| **Collapse** | Toggle panel between full height (h=2+) and header-only (h=1, ~60px). Collapsed panels show title + stats but no chart. |

### 3. Layout Persistence

**localStorage key:** `host-dashboard-layout-v2`

```typescript
interface PersistedDashboardState {
  layouts: ResponsiveLayouts       // panel positions/sizes per breakpoint
  collapsed: Record<string, boolean>  // which panels are collapsed
}
```

**Persistence rules:**
- Save on every `onLayoutChange` callback from react-grid-layout
- Save collapsed state on toggle
- Load on mount; fall back to `DEFAULT_LAYOUTS` if missing/corrupt
- Replace the Phase 1 `host-dashboard-sections` localStorage key (migration: delete old key on first load)

### 4. Dashboard Toolbar

Add a toolbar row between the time range selector and the chart grid:

```
[1h] [6h] [24h] [72h]                    [🔒 Lock Layout] [↺ Reset Layout]
```

| Button | Behaviour |
|---|---|
| **Lock Layout** (`Lock`/`Unlock` icon) | Toggle. When locked: drag handles hidden, resize disabled, panels static. Prevents accidental rearrangement. State saved to localStorage. |
| **Reset Layout** (`RotateCcw` icon) | Resets to `DEFAULT_LAYOUTS`, clears collapsed state, clears localStorage. Confirm with browser `confirm()` dialog. |

### 5. Maximized Panel Overlay

When a panel is maximized:

```
┌─────────────────────────────────────────────────┐
│ Sticky Health Header                            │  ← stays visible
│ Live Stats Bar                                  │  ← stays visible
├─────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────┐ │
│ │ ⤡ Latency & Loss    now 12.3  avg 11.2  ✕ │ │  ← maximize icon becomes "restore"
│ ├─────────────────────────────────────────────┤ │
│ │                                             │ │
│ │       [Recharts LineChart — full size]       │ │  ← height: calc(100vh - header)
│ │                                             │ │
│ └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

- Rendered as a `fixed` overlay (`z-30`) below the sticky header (`z-20`)
- Background: `bg-zinc-950` (matches dashboard)
- Escape key closes it
- Only one panel maximized at a time
- The grid underneath remains mounted but hidden (`opacity-0 pointer-events-none`) to avoid layout thrash

---

## File Structure

### Phase 2A — Single-file (first iteration)

Keep everything in `page.tsx` as today. Extract the grid layout logic and panel chrome into inline components within the same file. Expected size: ~800-900 lines.

### Phase 2B — Component extraction (follow-up, optional)

If the file grows beyond comfort, split into:

```
web/src/app/hosts/[id]/
├── page.tsx                    # Main page, data fetching, state
├── _components/
│   ├── dashboard-grid.tsx      # react-grid-layout wrapper + layout state
│   ├── chart-panel.tsx         # Panel chrome (drag handle, maximize, collapse)
│   ├── chart-configs.ts        # PANEL_CONFIGS array, DEFAULT_LAYOUTS
│   ├── maximized-overlay.tsx   # Full-screen chart overlay
│   └── dashboard-toolbar.tsx   # Lock/Reset buttons
```

---

## Implementation Plan

### Step 1: CSS Setup
- Import `react-grid-layout/css/styles.css` and `react-resizable/css/styles.css` in `page.tsx`
- Add Tailwind overrides for RGL classes to match dark theme (`.react-grid-item`, `.react-grid-placeholder`, `.react-resizable-handle`)

### Step 2: Panel Config Data Structure
- Define `PANEL_CONFIGS` array describing all 6 panels: id, title, stat keys, render function for chart content
- Define `DEFAULT_LAYOUTS` for all breakpoints
- This replaces the current inline `CollapsibleSection` > `DashboardChart` nesting

### Step 3: Replace Chart Area with ResponsiveGridLayout
- Use `useContainerWidth` hook for width measurement
- Use `useResponsiveLayout` or `<ResponsiveGridLayout>` from react-grid-layout v2
- Render each panel as a keyed `<div>` child containing the `ChartPanel` component
- Wire `onLayoutChange` to persist to localStorage

### Step 4: Panel Chrome Component
- Wrap each chart in a panel chrome with drag handle, title bar, stats, maximize/collapse buttons
- Set `dragConfig={{ handle: '.drag-handle' }}` on the grid so only the handle initiates drag
- Collapse toggles the layout item's `h` between 1 and its stored full height

### Step 5: Maximize Overlay
- `maximizedPanel` state (string | null) on the page
- When set, render a fixed overlay with the chart content at full size
- Escape key listener to close
- Maximize button in panel header toggles the state

### Step 6: Dashboard Toolbar
- Add Lock/Reset buttons to the time range row
- Lock sets all items to `static: true` and hides drag handles
- Reset calls `setLayouts(DEFAULT_LAYOUTS)` and clears localStorage

### Step 7: Styling & Polish
- Style RGL placeholder (the blue ghost shown during drag) to match dark theme
- Style resize handle to be subtle (small `se` corner indicator)
- Transitions on panel collapse/expand
- Ensure `ResponsiveContainer` in Recharts re-renders correctly when panel resizes (RGL triggers resize events → `ResponsiveContainer` picks up via ResizeObserver)

### Step 8: Remove Phase 1 Leftovers
- Remove `CollapsibleSection` component
- Remove `SectionKey` type, `getSectionState`, `saveSectionState`, section toggle logic
- Remove `host-dashboard-sections` localStorage key

---

## Technical Considerations

### Recharts + react-grid-layout Resize
- `ResponsiveContainer` uses its own ResizeObserver — it will automatically resize charts when panels are resized via RGL. No extra work needed.
- During drag, chart content stays rendered (no flicker).

### Performance
- 6 panels with 6 Recharts instances is lightweight. No virtualisation needed.
- `useMemo` on `chartData` already prevents recalculation on layout changes.
- RGL uses CSS transforms for positioning — no layout thrashing.

### syncId Compatibility
- All `LineChart` components keep `syncId="host-dashboard"`. This works across panels in the grid and in the maximized overlay (shared tooltip crosshair).

### Mobile
- On `xs`/`sm` breakpoints: single column, drag still works but resize is limited to height only (panels are full-width at `w: 6` in a 6-col grid).
- Lock layout by default on mobile? Consider in Phase 2B.

### New Lucide Icons Needed
- `GripVertical` — drag handle
- `Maximize2` / `Minimize2` — maximize/restore
- `Lock` / `Unlock` — layout lock toggle
- `RotateCcw` — reset layout
- `ChevronUp` — collapse panel (existing `ChevronDown`/`ChevronRight` already imported)

---

## Acceptance Criteria

- [ ] All 6 chart panels render inside react-grid-layout
- [ ] Panels can be dragged by their handle and reorder with animation
- [ ] Panels can be resized from the bottom-right corner; charts reflow
- [ ] Maximize button opens a full-area overlay with the chart; Escape/button closes it
- [ ] Collapse button shrinks panel to header-only; expand restores it
- [ ] Layout persists to localStorage and restores on reload
- [ ] Lock toggle disables drag/resize; Reset restores default layout
- [ ] Responsive: 2-col on desktop, 1-col on mobile
- [ ] All Phase 1 features remain functional (health header, live stats, live/pause, host info)
- [ ] TypeScript compiles cleanly (`tsc --noEmit`)
- [ ] `next build` succeeds
- [ ] Dark theme consistent — no unstyled RGL elements
