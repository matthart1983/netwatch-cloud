'use client'

import { useState, useEffect, useCallback, useMemo, useRef } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import { getHost, getMetrics, Host, MetricPoint } from '@/lib/api'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'
import { ChevronDown, ChevronRight, Activity, Pause, Circle } from 'lucide-react'

type TimeRange = '1h' | '6h' | '24h' | '72h'

const RANGES: { label: string; value: TimeRange; hours: number }[] = [
  { label: '1h', value: '1h', hours: 1 },
  { label: '6h', value: '6h', hours: 6 },
  { label: '24h', value: '24h', hours: 24 },
  { label: '72h', value: '72h', hours: 72 },
]

const TOOLTIP_STYLE = { background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }

type SectionKey = 'network' | 'system' | 'storage'

function getSectionState(): Record<SectionKey, boolean> {
  if (typeof window === 'undefined') return { network: true, system: true, storage: true }
  try {
    const stored = localStorage.getItem('host-dashboard-sections')
    if (stored) return JSON.parse(stored)
  } catch {}
  return { network: true, system: true, storage: true }
}

function saveSectionState(state: Record<SectionKey, boolean>) {
  try {
    localStorage.setItem('host-dashboard-sections', JSON.stringify(state))
  } catch {}
}

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

function computeStats(data: Record<string, unknown>[], key: string): { current: number | null; avg: number | null; max: number | null; min: number | null } {
  const vals = data.map(d => d[key]).filter((v): v is number => typeof v === 'number')
  if (vals.length === 0) return { current: null, avg: null, max: null, min: null }
  return {
    current: vals[vals.length - 1],
    avg: vals.reduce((a, b) => a + b, 0) / vals.length,
    max: Math.max(...vals),
    min: Math.min(...vals),
  }
}

type HealthStatus = 'healthy' | 'warning' | 'critical'

interface HealthResult {
  status: HealthStatus
  issues: string[]
  alertCount: number
}

function evaluateHealth(latest: MetricPoint | null, cpuCores: number | null): HealthResult {
  if (!latest) return { status: 'healthy', issues: [], alertCount: 0 }
  const issues: string[] = []
  let hasCritical = false
  let hasWarning = false

  if (latest.cpu_usage_pct != null) {
    if (latest.cpu_usage_pct > 95) {
      hasCritical = true
      issues.push(`CPU critical at ${latest.cpu_usage_pct.toFixed(1)}%`)
    } else if (latest.cpu_usage_pct > 80) {
      hasWarning = true
      issues.push(`CPU high at ${latest.cpu_usage_pct.toFixed(1)}%`)
    }
  }

  if (latest.memory_used_bytes != null && latest.memory_available_bytes != null) {
    const total = latest.memory_used_bytes + latest.memory_available_bytes
    const pct = total > 0 ? (latest.memory_used_bytes / total) * 100 : 0
    if (pct > 85) {
      hasWarning = true
      issues.push(`Memory at ${pct.toFixed(1)}%`)
    }
  }

  if (latest.disk_usage_pct != null && latest.disk_usage_pct > 90) {
    hasWarning = true
    issues.push(`Disk at ${latest.disk_usage_pct.toFixed(1)}%`)
  }

  if (latest.load_avg_1m != null && cpuCores != null && cpuCores > 0) {
    if (latest.load_avg_1m > cpuCores) {
      hasWarning = true
      issues.push(`Load ${latest.load_avg_1m.toFixed(2)} > ${cpuCores} cores`)
    }
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
  const [sections, setSections] = useState<Record<SectionKey, boolean>>(getSectionState)
  const [hostInfoOpen, setHostInfoOpen] = useState(false)
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null)

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

  // Live countdown for "last seen" and "data freshness"
  useEffect(() => {
    const tick = setInterval(() => {
      setSecondsAgo(prev => prev + 1)
    }, 1000)
    return () => clearInterval(tick)
  }, [])

  useEffect(() => {
    setSecondsAgo(0)
  }, [lastFetch])

  const toggleSection = (key: SectionKey) => {
    setSections(prev => {
      const next = { ...prev, [key]: !prev[key] }
      saveSectionState(next)
      return next
    })
  }

  // Deduplicate by time label — keep last point per minute for cleaner charts
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
    <div className="pb-8">
      {/* === Health Summary Header === */}
      <div className="sticky top-0 z-20 bg-zinc-950/95 backdrop-blur border-b border-zinc-800 -mx-4 px-4 py-3 mb-4">
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
              <span className="text-xs text-zinc-500">
                Last seen {lastSeenSecs}s ago
              </span>
            )}
            {/* Live/Pause indicator */}
            <button
              onClick={() => setPaused(p => !p)}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border transition-colors ${
                paused
                  ? 'bg-zinc-800 text-zinc-400 border-zinc-700 hover:border-zinc-600'
                  : 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30 hover:bg-emerald-500/25'
              }`}
            >
              {paused ? (
                <><Pause size={12} /> Paused</>
              ) : (
                <><Circle size={8} fill="currentColor" className="animate-pulse" /> Live</>
              )}
            </button>
            {lastFetch && (
              <span className="text-xs text-zinc-600">
                {secondsAgo}s ago
              </span>
            )}
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
        <LiveStatCard
          label="CPU"
          value={cpuStats.current != null ? `${cpuStats.current.toFixed(1)}%` : '—'}
          delta={getDeltaInfo(cpuStats.current, cpuStats.avg)}
          valueColor={getStatColor(cpuStats.current, 80, 95)}
        />
        <LiveStatCard
          label="Memory"
          value={memPct != null ? `${memPct.toFixed(1)}%` : '—'}
          delta={getDeltaInfo(memPct, memAvgPct)}
          valueColor={getStatColor(memPct, 85)}
        />
        <LiveStatCard
          label="Load 1m"
          value={loadStats.current != null ? loadStats.current.toFixed(2) : '—'}
          delta={getDeltaInfo(loadStats.current, loadStats.avg)}
          valueColor={getStatColor(loadStats.current, host.cpu_cores ?? 999)}
        />
        <LiveStatCard
          label="Disk"
          value={diskStats.current != null ? `${diskStats.current.toFixed(1)}%` : '—'}
          delta={getDeltaInfo(diskStats.current, diskStats.avg)}
          valueColor={getStatColor(diskStats.current, 90)}
        />
        <LiveStatCard
          label="Net RX/TX"
          value={rxStats.current != null && txStats.current != null
            ? `${formatRate(rxStats.current)} / ${formatRate(txStats.current)}`
            : '—'}
          delta={getDeltaInfo(
            rxStats.current != null && txStats.current != null ? rxStats.current + txStats.current : null,
            rxStats.avg != null && txStats.avg != null ? rxStats.avg + txStats.avg : null
          )}
          valueColor="text-zinc-100"
        />
        <LiveStatCard
          label="Connections"
          value={connStats.current != null ? connStats.current.toFixed(0) : '—'}
          delta={getDeltaInfo(connStats.current, connStats.avg)}
          valueColor="text-zinc-100"
        />
      </div>

      {/* === Time Range Selector === */}
      <div className="flex gap-2 mb-4">
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

      {chartData.length === 0 ? (
        <p className="text-zinc-400">No data for this time range.</p>
      ) : (
        <div className="space-y-6">
          {/* === Network Health Section === */}
          <CollapsibleSection
            title="Network Health"
            sectionKey="network"
            open={sections.network}
            onToggle={() => toggleSection('network')}
          >
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <DashboardChart title="Latency & Loss" data={chartData} stats={[
                { key: 'gateway_rtt', label: 'Gateway RTT' },
                { key: 'loss', label: 'Loss %' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="gateway_rtt" stroke="#34d399" dot={false} connectNulls strokeWidth={1.5} name="Gateway RTT (ms)" />
                    <Line dataKey="dns_rtt" stroke="#60a5fa" dot={false} connectNulls strokeWidth={1.5} name="DNS RTT (ms)" />
                    <Line dataKey="loss" stroke="#f87171" dot={false} connectNulls strokeWidth={1.5} name="Loss %" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>

              <DashboardChart title="Network & Connections" data={chartData} stats={[
                { key: 'net_rx', label: 'RX (KB)' },
                { key: 'net_tx', label: 'TX (KB)' },
                { key: 'connections', label: 'Connections' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="net_rx" stroke="#34d399" dot={false} connectNulls strokeWidth={1.5} name="RX (KB)" />
                    <Line dataKey="net_tx" stroke="#60a5fa" dot={false} connectNulls strokeWidth={1.5} name="TX (KB)" />
                    <Line dataKey="connections" stroke="#a78bfa" dot={false} connectNulls strokeWidth={1.5} name="Connections" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>
            </div>
          </CollapsibleSection>

          {/* === System Resources Section === */}
          <CollapsibleSection
            title="System Resources"
            sectionKey="system"
            open={sections.system}
            onToggle={() => toggleSection('system')}
          >
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <DashboardChart title="CPU & Memory" data={chartData} stats={[
                { key: 'cpu', label: 'CPU %' },
                { key: 'mem_used', label: 'Mem Used (GB)' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="cpu" stroke="#fbbf24" dot={false} connectNulls strokeWidth={1.5} name="CPU %" />
                    <Line dataKey="mem_used" stroke="#f472b6" dot={false} connectNulls strokeWidth={1.5} name="Used (GB)" />
                    <Line dataKey="mem_avail" stroke="#38bdf8" dot={false} connectNulls strokeWidth={1.5} name="Available (GB)" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>

              <DashboardChart title="Load & Swap" data={chartData} stats={[
                { key: 'load_1m', label: 'Load 1m' },
                { key: 'swap_used', label: 'Swap (MB)' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="load_1m" stroke="#34d399" dot={false} connectNulls strokeWidth={1.5} name="1m" />
                    <Line dataKey="load_5m" stroke="#fbbf24" dot={false} connectNulls strokeWidth={1.5} name="5m" />
                    <Line dataKey="load_15m" stroke="#f87171" dot={false} connectNulls strokeWidth={1.5} name="15m" />
                    <Line dataKey="swap_used" stroke="#f97316" dot={false} connectNulls strokeWidth={1.5} name="Swap (MB)" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>
            </div>
          </CollapsibleSection>

          {/* === Storage Section === */}
          <CollapsibleSection
            title="Storage"
            sectionKey="storage"
            open={sections.storage}
            onToggle={() => toggleSection('storage')}
          >
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
              <DashboardChart title="Disk Utilisation" data={chartData} stats={[
                { key: 'disk_usage', label: 'Disk %' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} domain={[0, 100]} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="disk_usage" stroke="#f97316" dot={false} connectNulls strokeWidth={1.5} name="Disk %" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>

              <DashboardChart title="TCP Connection States" data={chartData} stats={[
                { key: 'time_wait', label: 'TIME_WAIT' },
                { key: 'close_wait', label: 'CLOSE_WAIT' },
              ]}>
                <ResponsiveContainer width="100%" height={200}>
                  <LineChart data={chartData} syncId="host-dashboard">
                    <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                    <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                    <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                    <Tooltip contentStyle={TOOLTIP_STYLE} />
                    <Line dataKey="time_wait" stroke="#fbbf24" dot={false} connectNulls strokeWidth={1.5} name="TIME_WAIT" />
                    <Line dataKey="close_wait" stroke="#f87171" dot={false} connectNulls strokeWidth={1.5} name="CLOSE_WAIT" />
                  </LineChart>
                </ResponsiveContainer>
              </DashboardChart>
            </div>
          </CollapsibleSection>
        </div>
      )}
    </div>
  )
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded p-3">
      <div className="text-xs text-zinc-500">{label}</div>
      <div className="text-sm font-medium truncate">{value}</div>
    </div>
  )
}

function LiveStatCard({ label, value, delta, valueColor }: {
  label: string
  value: string
  delta: { arrow: string; color: string }
  valueColor: string
}) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-3">
      <div className="text-xs text-zinc-500 mb-1">{label}</div>
      <div className={`text-lg font-semibold ${valueColor}`}>{value}</div>
      <div className={`text-xs mt-0.5 ${delta.color}`}>{delta.arrow}</div>
    </div>
  )
}

function CollapsibleSection({ title, sectionKey, open, onToggle, children }: {
  title: string
  sectionKey: string
  open: boolean
  onToggle: () => void
  children: React.ReactNode
}) {
  return (
    <div>
      <button
        onClick={onToggle}
        className="flex items-center gap-2 mb-3 text-sm font-medium text-zinc-300 hover:text-zinc-100 transition-colors"
      >
        {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
        {title}
      </button>
      {open && children}
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

function DashboardChart({ title, data, stats, children }: {
  title: string
  data: Record<string, unknown>[]
  stats: { key: string; label: string }[]
  children: React.ReactNode
}) {
  const primaryStat = stats[0] ? computeStats(data, stats[0].key) : null

  const formatVal = (v: number | null) => {
    if (v == null) return '—'
    if (Math.abs(v) >= 1000) return v.toFixed(0)
    if (Math.abs(v) >= 100) return v.toFixed(1)
    return v.toFixed(2)
  }

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-sm font-medium text-zinc-300">{title}</h3>
        {primaryStat && (
          <div className="flex gap-3">
            <StatPill label="now" value={formatVal(primaryStat.current)} />
            <StatPill label="avg" value={formatVal(primaryStat.avg)} />
            <StatPill label="max" value={formatVal(primaryStat.max)} />
            <StatPill label="min" value={formatVal(primaryStat.min)} />
          </div>
        )}
      </div>
      {children}
    </div>
  )
}
