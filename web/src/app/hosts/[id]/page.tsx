'use client'

import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import { getHost, getMetrics, Host, MetricPoint } from '@/lib/api'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'

import {
  ChevronDown, ChevronRight, ChevronUp, Pause, Circle,
  GripVertical, Maximize2, Minimize2, Lock, Unlock, RotateCcw,
} from 'lucide-react'

// ─── Constants ───────────────────────────────────────────────

type TimeRange = '1h' | '6h' | '24h' | '72h'

const RANGES: { label: string; value: TimeRange; hours: number }[] = [
  { label: '1h', value: '1h', hours: 1 },
  { label: '6h', value: '6h', hours: 6 },
  { label: '24h', value: '24h', hours: 24 },
  { label: '72h', value: '72h', hours: 72 },
]

const TOOLTIP_STYLE = { background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }

const PANEL_IDS = ['latency-loss', 'network-conn', 'cpu-memory', 'cpu-per-core', 'load-swap', 'disk-util', 'tcp-states'] as const
type PanelId = typeof PANEL_IDS[number]

const LS_KEY = 'host-dashboard-state-v3'

function loadCollapsed(): Record<string, boolean> {
  if (typeof window === 'undefined') return {}
  try {
    const raw = localStorage.getItem(LS_KEY)
    if (raw) return JSON.parse(raw).collapsed || {}
  } catch {}
  return {}
}

function saveCollapsed(collapsed: Record<string, boolean>) {
  try { localStorage.setItem(LS_KEY, JSON.stringify({ collapsed })) } catch {}
}

// ─── Panel Config ────────────────────────────────────────────

interface PanelConfig {
  id: PanelId
  title: string
  statKeys: { key: string; label: string }[]
  yDomain?: [number | string, number | string]
  lines: { dataKey: string; stroke: string; name: string }[]
}

const PANEL_CONFIGS: PanelConfig[] = [
  {
    id: 'latency-loss',
    title: 'Latency & Loss',
    statKeys: [{ key: 'gateway_rtt', label: 'Gateway RTT' }, { key: 'loss', label: 'Loss %' }],
    lines: [
      { dataKey: 'gateway_rtt', stroke: '#34d399', name: 'Gateway RTT (ms)' },
      { dataKey: 'dns_rtt', stroke: '#60a5fa', name: 'DNS RTT (ms)' },
      { dataKey: 'loss', stroke: '#f87171', name: 'Loss %' },
    ],
  },
  {
    id: 'network-conn',
    title: 'Network & Connections',
    statKeys: [{ key: 'net_rx', label: 'RX (KB)' }, { key: 'net_tx', label: 'TX (KB)' }, { key: 'connections', label: 'Connections' }],
    lines: [
      { dataKey: 'net_rx', stroke: '#34d399', name: 'RX (KB)' },
      { dataKey: 'net_tx', stroke: '#60a5fa', name: 'TX (KB)' },
      { dataKey: 'connections', stroke: '#a78bfa', name: 'Connections' },
    ],
  },
  {
    id: 'cpu-memory',
    title: 'CPU & Memory',
    statKeys: [{ key: 'cpu', label: 'CPU %' }, { key: 'mem_used', label: 'Mem Used (GB)' }],
    lines: [
      { dataKey: 'cpu', stroke: '#fbbf24', name: 'CPU %' },
      { dataKey: 'mem_used', stroke: '#f472b6', name: 'Used (GB)' },
      { dataKey: 'mem_avail', stroke: '#38bdf8', name: 'Available (GB)' },
    ],
  },
  {
    id: 'cpu-per-core',
    title: 'CPU per Core',
    statKeys: [],
    yDomain: [0, 100],
    lines: [],  // dynamic — generated at render time
  },
  {
    id: 'load-swap',
    title: 'Load & Swap',
    statKeys: [{ key: 'load_1m', label: 'Load 1m' }, { key: 'swap_used', label: 'Swap (MB)' }],
    lines: [
      { dataKey: 'load_1m', stroke: '#34d399', name: '1m' },
      { dataKey: 'load_5m', stroke: '#fbbf24', name: '5m' },
      { dataKey: 'load_15m', stroke: '#f87171', name: '15m' },
      { dataKey: 'swap_used', stroke: '#f97316', name: 'Swap (MB)' },
    ],
  },
  {
    id: 'disk-util',
    title: 'Disk Utilisation',
    statKeys: [{ key: 'disk_usage', label: 'Disk %' }],
    yDomain: [0, 100],
    lines: [
      { dataKey: 'disk_usage', stroke: '#f97316', name: 'Disk %' },
    ],
  },
  {
    id: 'tcp-states',
    title: 'TCP Connection States',
    statKeys: [{ key: 'time_wait', label: 'TIME_WAIT' }, { key: 'close_wait', label: 'CLOSE_WAIT' }],
    lines: [
      { dataKey: 'time_wait', stroke: '#fbbf24', name: 'TIME_WAIT' },
      { dataKey: 'close_wait', stroke: '#f87171', name: 'CLOSE_WAIT' },
    ],
  },
]

// ─── Utility Functions ───────────────────────────────────────

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

function formatUptime(secs: number): string {
  const days = Math.floor(secs / 86400)
  const hours = Math.floor((secs % 86400) / 3600)
  if (days > 0) return `${days}d ${hours}h`
  const mins = Math.floor((secs % 3600) / 60)
  return `${hours}h ${mins}m`
}

function formatRate(kb: number): string {
  if (kb >= 1024 * 1024) return `${(kb / (1024 * 1024)).toFixed(1)} GB/s`
  if (kb >= 1024) return `${(kb / 1024).toFixed(1)} MB/s`
  return `${kb.toFixed(0)} KB/s`
}

function timeAgo(iso: string): number {
  return Math.max(0, Math.floor((Date.now() - new Date(iso).getTime()) / 1000))
}

function computeStats(data: Record<string, unknown>[], key: string) {
  const vals = data.map(d => d[key]).filter((v): v is number => typeof v === 'number')
  if (vals.length === 0) return { current: null, avg: null, max: null, min: null }
  return {
    current: vals[vals.length - 1],
    avg: vals.reduce((a, b) => a + b, 0) / vals.length,
    max: Math.max(...vals),
    min: Math.min(...vals),
  }
}

function formatStatVal(v: number | null): string {
  if (v == null) return '—'
  if (Math.abs(v) >= 1000) return v.toFixed(0)
  if (Math.abs(v) >= 100) return v.toFixed(1)
  return v.toFixed(2)
}

type HealthStatus = 'healthy' | 'warning' | 'critical'

interface HealthResult { status: HealthStatus; issues: string[]; alertCount: number }

function evaluateHealth(latest: MetricPoint | null, cpuCores: number | null): HealthResult {
  if (!latest) return { status: 'healthy', issues: [], alertCount: 0 }
  const issues: string[] = []
  let hasCritical = false
  let hasWarning = false

  if (latest.cpu_usage_pct != null) {
    if (latest.cpu_usage_pct > 95) { hasCritical = true; issues.push(`CPU critical at ${latest.cpu_usage_pct.toFixed(1)}%`) }
    else if (latest.cpu_usage_pct > 80) { hasWarning = true; issues.push(`CPU high at ${latest.cpu_usage_pct.toFixed(1)}%`) }
  }
  if (latest.memory_used_bytes != null && latest.memory_available_bytes != null) {
    const total = latest.memory_used_bytes + latest.memory_available_bytes
    const pct = total > 0 ? (latest.memory_used_bytes / total) * 100 : 0
    if (pct > 85) { hasWarning = true; issues.push(`Memory at ${pct.toFixed(1)}%`) }
  }
  if (latest.disk_usage_pct != null && latest.disk_usage_pct > 90) { hasWarning = true; issues.push(`Disk at ${latest.disk_usage_pct.toFixed(1)}%`) }
  if (latest.load_avg_1m != null && cpuCores != null && cpuCores > 0 && latest.load_avg_1m > cpuCores) {
    hasWarning = true; issues.push(`Load ${latest.load_avg_1m.toFixed(2)} > ${cpuCores} cores`)
  }

  const status: HealthStatus = hasCritical ? 'critical' : hasWarning ? 'warning' : 'healthy'
  return { status, issues, alertCount: issues.length }
}

function getDeltaInfo(current: number | null, avg: number | null): { arrow: string; color: string } {
  if (current == null || avg == null || avg === 0) return { arrow: '—', color: 'text-zinc-500' }
  const diff = ((current - avg) / avg) * 100
  if (Math.abs(diff) < 1) return { arrow: '→', color: 'text-zinc-400' }
  if (diff > 0) return { arrow: `▲ ${diff.toFixed(0)}%`, color: 'text-red-400' }
  return { arrow: `▼ ${Math.abs(diff).toFixed(0)}%`, color: 'text-emerald-400' }
}

function getStatColor(value: number | null, warnThreshold: number, critThreshold?: number): string {
  if (value == null) return 'text-zinc-400'
  if (critThreshold != null && value > critThreshold) return 'text-red-400'
  if (value > warnThreshold) return 'text-yellow-400'
  return 'text-emerald-400'
}

// ─── Main Page Component ─────────────────────────────────────

export default function HostDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { token, isLoading: authLoading } = useAuth()
  const router = useRouter()
  const [host, setHost] = useState<Host | null>(null)
  const [points, setPoints] = useState<MetricPoint[]>([])
  const [range, setRange] = useState<TimeRange>('24h')
  const [loading, setLoading] = useState(true)
  const [paused, setPaused] = useState(false)
  const [lastFetch, setLastFetch] = useState<Date | null>(null)
  const [secondsAgo, setSecondsAgo] = useState(0)
  const [hostInfoOpen, setHostInfoOpen] = useState(false)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

  // Panel state
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>(() => loadCollapsed())
  const [locked, setLocked] = useState(false)
  const [maximizedPanel, setMaximizedPanel] = useState<PanelId | null>(null)

  const toggleCollapse = useCallback((panelId: string) => {
    setCollapsed(prev => {
      const next = { ...prev, [panelId]: !prev[panelId] }
      saveCollapsed(next)
      return next
    })
  }, [])

  const toggleLock = useCallback(() => setLocked(prev => !prev), [])

  const resetLayout = useCallback(() => {
    setCollapsed({})
    setLocked(false)
    saveCollapsed({})
  }, [])

  // Data fetching
  const fetchData = useCallback(async () => {
    if (!token || !id) return
    try {
      const hours = RANGES.find(r => r.value === range)?.hours || 24
      const from = new Date(Date.now() - hours * 3600 * 1000).toISOString()
      const [h, m] = await Promise.all([getHost(id), getMetrics(id, from)])
      setHost(h)
      setPoints(m.points)
      setLastFetch(new Date())
    } catch {
      router.push('/')
    } finally {
      setLoading(false)
    }
  }, [token, id, range, router])

  useEffect(() => {
    if (authLoading) return
    if (!token) { router.push('/login'); return }
    fetchData()
  }, [authLoading, token, fetchData, router])

  useEffect(() => {
    if (paused || authLoading || !token) {
      if (intervalRef.current) { clearInterval(intervalRef.current); intervalRef.current = null }
      return
    }
    intervalRef.current = setInterval(fetchData, 15_000)
    return () => { if (intervalRef.current) clearInterval(intervalRef.current) }
  }, [paused, authLoading, token, fetchData])

  useEffect(() => {
    const tick = setInterval(() => setSecondsAgo(prev => prev + 1), 1000)
    return () => clearInterval(tick)
  }, [])

  useEffect(() => { setSecondsAgo(0) }, [lastFetch])

  // Escape key to close maximized panel
  useEffect(() => {
    if (!maximizedPanel) return
    const handler = (e: KeyboardEvent) => { if (e.key === 'Escape') setMaximizedPanel(null) }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [maximizedPanel])

  // Chart data (deduplicated)
  const chartData = useMemo(() => {
    const result: Record<string, unknown>[] = []
    const seen = new Set<string>()
    for (let i = points.length - 1; i >= 0; i--) {
      const p = points[i]
      const label = new Date(p.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
      if (seen.has(label)) continue
      seen.add(label)
      result.unshift({
        time: label,
        gateway_rtt: p.gateway_rtt_ms ?? undefined,
        dns_rtt: p.dns_rtt_ms ?? undefined,
        loss: p.gateway_loss_pct ?? undefined,
        connections: p.connection_count ?? undefined,
        cpu: p.cpu_usage_pct ?? undefined,
        mem_used: p.memory_used_bytes != null ? p.memory_used_bytes / (1024 * 1024 * 1024) : undefined,
        mem_avail: p.memory_available_bytes != null ? p.memory_available_bytes / (1024 * 1024 * 1024) : undefined,
        load_1m: p.load_avg_1m ?? undefined,
        load_5m: p.load_avg_5m ?? undefined,
        load_15m: p.load_avg_15m ?? undefined,
        swap_used: p.swap_used_bytes != null ? p.swap_used_bytes / (1024 * 1024) : undefined,
        disk_read: p.disk_read_bytes != null ? p.disk_read_bytes / (1024 * 1024) : undefined,
        disk_write: p.disk_write_bytes != null ? p.disk_write_bytes / (1024 * 1024) : undefined,
        disk_usage: p.disk_usage_pct ?? undefined,
        time_wait: p.tcp_time_wait ?? undefined,
        close_wait: p.tcp_close_wait ?? undefined,
        net_rx: p.net_rx_bytes != null ? p.net_rx_bytes / 1024 : undefined,
        net_tx: p.net_tx_bytes != null ? p.net_tx_bytes / 1024 : undefined,
        ...(p.cpu_per_core ? Object.fromEntries(p.cpu_per_core.map((v, i) => [`core_${i}`, v])) : {}),
      })
    }
    return result
  }, [points])

  const latest = points.length > 0 ? points[points.length - 1] : null
  const health = useMemo(() => evaluateHealth(latest, host?.cpu_cores ?? null), [latest, host?.cpu_cores])

  // Stats for live bar
  const cpuStats = useMemo(() => computeStats(chartData, 'cpu'), [chartData])
  const memPct = useMemo(() => {
    if (!latest?.memory_used_bytes || !latest?.memory_available_bytes) return null
    const total = latest.memory_used_bytes + latest.memory_available_bytes
    return total > 0 ? (latest.memory_used_bytes / total) * 100 : 0
  }, [latest])
  const memAvgPct = useMemo(() => {
    const vals = points.filter(p => p.memory_used_bytes != null && p.memory_available_bytes != null)
      .map(p => { const t = p.memory_used_bytes! + p.memory_available_bytes!; return t > 0 ? (p.memory_used_bytes! / t) * 100 : 0 })
    return vals.length > 0 ? vals.reduce((a, b) => a + b, 0) / vals.length : null
  }, [points])
  const loadStats = useMemo(() => computeStats(chartData, 'load_1m'), [chartData])
  const diskStats = useMemo(() => computeStats(chartData, 'disk_usage'), [chartData])
  const rxStats = useMemo(() => computeStats(chartData, 'net_rx'), [chartData])
  const txStats = useMemo(() => computeStats(chartData, 'net_tx'), [chartData])
  const connStats = useMemo(() => computeStats(chartData, 'connections'), [chartData])

  if (loading || !host) {
    return <div className="text-zinc-400 mt-10">Loading...</div>
  }

  const lastSeenSecs = host.last_seen_at ? timeAgo(host.last_seen_at) + secondsAgo : null
  const statusColors: Record<HealthStatus, string> = {
    healthy: 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30',
    warning: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    critical: 'bg-red-500/20 text-red-400 border-red-500/30',
  }

  return (
    <div className="pb-8" style={{ width: '100vw', marginLeft: 'calc(-50vw + 50%)', paddingLeft: '1.5rem', paddingRight: '1.5rem' }}>
      {/* === Health Summary Header === */}
      <div className="sticky top-0 z-20 bg-zinc-950/95 backdrop-blur border-b border-zinc-800 -mx-6 px-6 py-3 mb-4">
        <div className="flex items-center justify-between flex-wrap gap-3">
          <div className="flex items-center gap-3">
            <button onClick={() => router.push('/')} className="text-zinc-400 hover:text-zinc-100 text-lg">←</button>
            <h1 className="text-lg font-bold truncate">{host.hostname}</h1>
            <span className={`px-2.5 py-0.5 rounded-full text-xs font-medium border ${statusColors[health.status]}`}>
              {health.status === 'healthy' ? 'Healthy' : health.status === 'warning' ? 'Warning' : 'Critical'}
            </span>
            {health.alertCount > 0 && (
              <span className="px-2 py-0.5 rounded-full text-xs font-medium bg-red-500/20 text-red-400 border border-red-500/30">
                {health.alertCount} {health.alertCount === 1 ? 'issue' : 'issues'}
              </span>
            )}
          </div>
          <div className="flex items-center gap-4">
            {health.issues.length > 0 && (
              <span className="text-xs text-yellow-400 hidden md:block truncate max-w-xs">
                {health.issues[0]}
              </span>
            )}
            {lastSeenSecs != null && (
              <span className="text-xs text-zinc-500 tabular-nums min-w-[7rem] text-right">Last seen {lastSeenSecs}s ago</span>
            )}
            <button
              onClick={() => setPaused(p => !p)}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-colors ${
                paused
                  ? 'bg-zinc-800 text-zinc-400 border-zinc-700 hover:border-zinc-600'
                  : 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30 hover:bg-emerald-500/25'
              }`}
            >
              {paused ? <><Pause size={12} /> Paused</> : <><Circle size={8} fill="currentColor" className="animate-pulse" /> Live</>}
            </button>
            {lastFetch && <span className="text-xs text-zinc-600 tabular-nums min-w-[3rem] text-right">{secondsAgo}s ago</span>}
          </div>
        </div>
      </div>

      {/* === Collapsible Host Info Panel === */}
      <div className="mb-4">
        <button
          onClick={() => setHostInfoOpen(o => !o)}
          className="flex items-center gap-2 text-sm text-zinc-400 hover:text-zinc-200 transition-colors"
        >
          {hostInfoOpen ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
          <span className="font-medium">Host Information</span>
          {!hostInfoOpen && (
            <span className="text-xs text-zinc-600 ml-2">
              {host.os || '—'} · {host.cpu_cores || '?'} cores · {host.memory_total_bytes ? formatBytes(host.memory_total_bytes) : '—'}
            </span>
          )}
        </button>
        {hostInfoOpen && (
          <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mt-3">
            <InfoItem label="Hostname" value={host.hostname} />
            <InfoItem label="OS" value={host.os || '—'} />
            <InfoItem label="Kernel" value={host.kernel || '—'} />
            <InfoItem label="CPU Model" value={host.cpu_model ? host.cpu_model.replace(/\(R\)|\(TM\)/g, '').split('@')[0].trim() : '—'} />
            <InfoItem label="Cores" value={host.cpu_cores?.toString() || '—'} />
            <InfoItem label="Memory" value={host.memory_total_bytes ? formatBytes(host.memory_total_bytes) : '—'} />
            <InfoItem label="Uptime" value={host.uptime_secs ? formatUptime(host.uptime_secs) : '—'} />
            <InfoItem label="Agent" value={host.agent_version || '—'} />
          </div>
        )}
      </div>

      {/* === Live Stats Bar === */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 mb-4">
        <LiveStatCard label="CPU" value={cpuStats.current != null ? `${cpuStats.current.toFixed(1)}%` : '—'} delta={getDeltaInfo(cpuStats.current, cpuStats.avg)} valueColor={getStatColor(cpuStats.current, 80, 95)} />
        <LiveStatCard label="Memory" value={memPct != null ? `${memPct.toFixed(1)}%` : '—'} delta={getDeltaInfo(memPct, memAvgPct)} valueColor={getStatColor(memPct, 85)} />
        <LiveStatCard label="Load 1m" value={loadStats.current != null ? loadStats.current.toFixed(2) : '—'} delta={getDeltaInfo(loadStats.current, loadStats.avg)} valueColor={getStatColor(loadStats.current, host.cpu_cores ?? 999)} />
        <LiveStatCard label="Disk" value={diskStats.current != null ? `${diskStats.current.toFixed(1)}%` : '—'} delta={getDeltaInfo(diskStats.current, diskStats.avg)} valueColor={getStatColor(diskStats.current, 90)} />
        <LiveStatCard label="Net RX/TX" value={rxStats.current != null && txStats.current != null ? `${formatRate(rxStats.current)} / ${formatRate(txStats.current)}` : '—'} delta={getDeltaInfo(rxStats.current != null && txStats.current != null ? rxStats.current + txStats.current : null, rxStats.avg != null && txStats.avg != null ? rxStats.avg + txStats.avg : null)} valueColor="text-zinc-100" />
        <LiveStatCard label="Connections" value={connStats.current != null ? connStats.current.toFixed(0) : '—'} delta={getDeltaInfo(connStats.current, connStats.avg)} valueColor="text-zinc-100" />
      </div>

      {/* === Time Range + Dashboard Toolbar === */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex gap-2">
          {RANGES.map(r => (
            <button
              key={r.value}
              onClick={() => { setRange(r.value); setLoading(true) }}
              className={`px-3 py-1 rounded text-sm ${range === r.value ? 'bg-emerald-600 text-white' : 'bg-zinc-800 text-zinc-400 hover:text-zinc-100'}`}
            >
              {r.label}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={toggleLock}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium transition-colors ${
              locked ? 'bg-yellow-500/15 text-yellow-400 border border-yellow-500/30' : 'bg-zinc-800 text-zinc-400 hover:text-zinc-100 border border-zinc-700'
            }`}
            title={locked ? 'Unlock panels' : 'Lock panels (prevent collapse)'}
          >
            {locked ? <Lock size={12} /> : <Unlock size={12} />}
            {locked ? 'Locked' : 'Unlocked'}
          </button>
          <button
            onClick={resetLayout}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded text-xs font-medium bg-zinc-800 text-zinc-400 hover:text-zinc-100 border border-zinc-700 transition-colors"
            title="Reset all panels"
          >
            <RotateCcw size={12} />
            Reset
          </button>
        </div>
      </div>

      {/* === Chart Grid === */}
      {chartData.length === 0 ? (
        <p className="text-zinc-400">No data for this time range.</p>
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
          {PANEL_CONFIGS.map(config => (
            <div key={config.id} style={{ height: collapsed[config.id] ? 'auto' : 280 }}>
              <ChartPanel
                config={config}
                data={chartData}
                isCollapsed={!!collapsed[config.id]}
                isLocked={locked}
                onToggleCollapse={() => toggleCollapse(config.id)}
                onMaximize={() => setMaximizedPanel(config.id)}
              />
            </div>
          ))}
        </div>
      )}

      {/* === Maximized Panel Overlay === */}
      {maximizedPanel && (
        <MaximizedOverlay
          config={PANEL_CONFIGS.find(c => c.id === maximizedPanel)!}
          data={chartData}
          onClose={() => setMaximizedPanel(null)}
        />
      )}
    </div>
  )
}

// ─── Sub-components ──────────────────────────────────────────

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded p-3">
      <div className="text-xs text-zinc-500">{label}</div>
      <div className="text-sm font-medium truncate">{value}</div>
    </div>
  )
}

function LiveStatCard({ label, value, delta, valueColor }: {
  label: string; value: string; delta: { arrow: string; color: string }; valueColor: string
}) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
      <div className="text-xs text-zinc-500 mb-1">{label}</div>
      <div className={`text-lg font-semibold ${valueColor}`}>{value}</div>
      <div className={`text-xs mt-0.5 ${delta.color}`}>{delta.arrow}</div>
    </div>
  )
}

const CORE_COLORS = ['#34d399', '#60a5fa', '#fbbf24', '#f87171', '#a78bfa', '#f472b6', '#fb923c', '#2dd4bf', '#e879f9', '#4ade80', '#38bdf8', '#facc15', '#f43f5e', '#818cf8', '#ec4899', '#f59e0b', '#14b8a6', '#8b5cf6']

function getCoreLines(data: Record<string, unknown>[]) {
  if (data.length === 0) return []
  // Scan all data points for core keys (first point might not have them)
  const coreKeySet = new Set<string>()
  for (const d of data) {
    for (const k of Object.keys(d)) {
      if (k.startsWith('core_')) coreKeySet.add(k)
    }
    if (coreKeySet.size > 0) break  // found cores, no need to keep scanning
  }
  const coreKeys = Array.from(coreKeySet).sort((a, b) => parseInt(a.split('_')[1]) - parseInt(b.split('_')[1]))
  return coreKeys.map((key, i) => ({
    dataKey: key,
    stroke: CORE_COLORS[i % CORE_COLORS.length],
    name: `Core ${key.split('_')[1]}`,
  }))
}

function ChartPanel({ config, data, isCollapsed, isLocked, onToggleCollapse, onMaximize }: {
  config: PanelConfig
  data: Record<string, unknown>[]
  isCollapsed: boolean
  isLocked: boolean
  onToggleCollapse: () => void
  onMaximize: () => void
}) {
  const primaryStat = config.statKeys[0] ? computeStats(data, config.statKeys[0].key) : null
  const lines = config.id === 'cpu-per-core' ? getCoreLines(data) : config.lines

  // Hide per-core panel if no core data
  if (config.id === 'cpu-per-core' && lines.length === 0) return null

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg h-full flex flex-col overflow-hidden">
      {/* Panel header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-zinc-800/50 shrink-0">
        {!isLocked && (
          <div className="panel-drag-handle cursor-grab active:cursor-grabbing text-zinc-600 hover:text-zinc-400 shrink-0">
            <GripVertical size={14} />
          </div>
        )}
        <h3 className="text-sm font-medium text-zinc-300 truncate">{config.title}</h3>
        {primaryStat && !isCollapsed && (
          <div className="hidden sm:flex gap-2 ml-auto mr-2">
            <StatPill label="now" value={formatStatVal(primaryStat.current)} />
            <StatPill label="avg" value={formatStatVal(primaryStat.avg)} />
            <StatPill label="max" value={formatStatVal(primaryStat.max)} />
            <StatPill label="min" value={formatStatVal(primaryStat.min)} />
          </div>
        )}
        <div className="flex items-center gap-1 ml-auto shrink-0">
          <button onClick={onMaximize} className="p-1 text-zinc-600 hover:text-zinc-300 transition-colors" title="Maximize">
            <Maximize2 size={13} />
          </button>
          <button onClick={onToggleCollapse} className="p-1 text-zinc-600 hover:text-zinc-300 transition-colors" title={isCollapsed ? 'Expand' : 'Collapse'}>
            {isCollapsed ? <ChevronDown size={13} /> : <ChevronUp size={13} />}
          </button>
        </div>
      </div>
      {/* Chart content */}
      {!isCollapsed && (
        <div className="flex-1 min-h-0 p-2" style={{ minHeight: 140 }}>
          <ResponsiveContainer width="100%" height="100%" minWidth={0} minHeight={120}>
            <LineChart data={data} syncId="host-dashboard">
              <CartesianGrid strokeDasharray="3 3" stroke="#333" />
              <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
              <YAxis stroke="#666" tick={{ fontSize: 11 }} domain={config.yDomain} />
              <Tooltip contentStyle={TOOLTIP_STYLE} />
              {lines.map(line => (
                <Line key={line.dataKey} dataKey={line.dataKey} stroke={line.stroke} dot={false} connectNulls strokeWidth={1.5} name={line.name} />
              ))}
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  )
}

function MaximizedOverlay({ config, data, onClose }: {
  config: PanelConfig; data: Record<string, unknown>[]; onClose: () => void
}) {
  const primaryStat = config.statKeys[0] ? computeStats(data, config.statKeys[0].key) : null
  const lines = config.id === 'cpu-per-core' ? getCoreLines(data) : config.lines

  return (
    <div className="fixed inset-0 z-30 bg-zinc-950/98 flex flex-col" onClick={onClose}>
      <div className="flex items-center gap-3 px-6 py-4 border-b border-zinc-800 shrink-0" onClick={e => e.stopPropagation()}>
        <h2 className="text-lg font-semibold text-zinc-200">{config.title}</h2>
        {primaryStat && (
          <div className="flex gap-3 ml-4">
            <StatPill label="now" value={formatStatVal(primaryStat.current)} />
            <StatPill label="avg" value={formatStatVal(primaryStat.avg)} />
            <StatPill label="max" value={formatStatVal(primaryStat.max)} />
            <StatPill label="min" value={formatStatVal(primaryStat.min)} />
          </div>
        )}
        <button onClick={onClose} className="ml-auto p-2 text-zinc-400 hover:text-zinc-100 transition-colors" title="Close (Escape)">
          <Minimize2 size={18} />
        </button>
      </div>
      <div className="flex-1 p-6" onClick={e => e.stopPropagation()}>
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={data} syncId="host-dashboard">
            <CartesianGrid strokeDasharray="3 3" stroke="#333" />
            <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 12 }} interval="preserveStartEnd" />
            <YAxis stroke="#666" tick={{ fontSize: 12 }} domain={config.yDomain} />
            <Tooltip contentStyle={TOOLTIP_STYLE} />
            {lines.map(line => (
              <Line key={line.dataKey} dataKey={line.dataKey} stroke={line.stroke} dot={false} connectNulls strokeWidth={2} name={line.name} />
            ))}
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}

function StatPill({ label, value }: { label: string; value: string }) {
  return (
    <span className="text-xs text-zinc-500">
      <span className="text-zinc-600">{label}</span> {value}
    </span>
  )
}
