'use client'

import { useState, useEffect, useCallback } from 'react'
import { useParams, useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import { getHost, getMetrics, Host, MetricPoint } from '@/lib/api'
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'

type TimeRange = '1h' | '6h' | '24h' | '72h'

const RANGES: { label: string; value: TimeRange; hours: number }[] = [
  { label: '1h', value: '1h', hours: 1 },
  { label: '6h', value: '6h', hours: 6 },
  { label: '24h', value: '24h', hours: 24 },
  { label: '72h', value: '72h', hours: 72 },
]

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

export default function HostDetailPage() {
  const { id } = useParams<{ id: string }>()
  const { token, isLoading: authLoading } = useAuth()
  const router = useRouter()
  const [host, setHost] = useState<Host | null>(null)
  const [points, setPoints] = useState<MetricPoint[]>([])
  const [range, setRange] = useState<TimeRange>('24h')
  const [loading, setLoading] = useState(true)

  const fetchData = useCallback(async () => {
    if (!token || !id) return
    try {
      const hours = RANGES.find(r => r.value === range)?.hours || 24
      const from = new Date(Date.now() - hours * 3600 * 1000).toISOString()
      const [h, m] = await Promise.all([getHost(id), getMetrics(id, from)])
      setHost(h)
      setPoints(m.points)
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
    const interval = setInterval(fetchData, 15_000)
    return () => clearInterval(interval)
  }, [authLoading, token, fetchData, router])

  if (loading || !host) {
    return <div className="text-zinc-400 mt-10">Loading...</div>
  }

  const chartData = points.map((p, i) => ({
    time: new Date(p.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    idx: i,
    gateway_rtt: p.gateway_rtt_ms ?? null,
    dns_rtt: p.dns_rtt_ms ?? null,
    loss: p.gateway_loss_pct ?? null,
    connections: p.connection_count ?? null,
    cpu: p.cpu_usage_pct ?? null,
    mem_used: p.memory_used_bytes != null ? p.memory_used_bytes / (1024 * 1024 * 1024) : null,
    mem_avail: p.memory_available_bytes != null ? p.memory_available_bytes / (1024 * 1024 * 1024) : null,
    load_1m: p.load_avg_1m ?? null,
    load_5m: p.load_avg_5m ?? null,
    load_15m: p.load_avg_15m ?? null,
    swap_used: p.swap_used_bytes != null ? p.swap_used_bytes / (1024 * 1024) : null,
    disk_read: p.disk_read_bytes != null ? p.disk_read_bytes / (1024 * 1024) : null,
    disk_write: p.disk_write_bytes != null ? p.disk_write_bytes / (1024 * 1024) : null,
    time_wait: p.tcp_time_wait ?? null,
    close_wait: p.tcp_close_wait ?? null,
  }))

  return (
    <div>
      <div className="flex items-center gap-3 mb-6">
        <button onClick={() => router.push('/')} className="text-zinc-400 hover:text-zinc-100">←</button>
        <span className={`w-3 h-3 rounded-full ${host.is_online ? 'bg-emerald-400' : 'bg-red-400'}`} />
        <h1 className="text-2xl font-bold">{host.hostname}</h1>
        <span className="text-zinc-400 text-sm">{host.is_online ? 'Online' : 'Offline'}</span>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mb-6">
        <Stat label="OS" value={host.os || '—'} />
        <Stat label="Kernel" value={host.kernel || '—'} />
        <Stat label="CPU" value={host.cpu_model ? host.cpu_model.replace(/\(R\)|\(TM\)/g, '').split('@')[0].trim() : '—'} />
        <Stat label="Cores" value={host.cpu_cores?.toString() || '—'} />
        <Stat label="Memory" value={host.memory_total_bytes ? formatBytes(host.memory_total_bytes) : '—'} />
        <Stat label="Uptime" value={host.uptime_secs ? formatUptime(host.uptime_secs) : '—'} />
      </div>

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
          <ChartCard title="Gateway Latency (ms)">
            <ResponsiveContainer width="100%" height={200}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="gateway_rtt" stroke="#34d399" dot={false} strokeWidth={1.5} name="Gateway RTT" connectNulls />
                <Line type="monotone" dataKey="dns_rtt" stroke="#60a5fa" dot={false} strokeWidth={1.5} name="DNS RTT" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Packet Loss (%)">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} domain={[0, 'auto']} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="loss" stroke="#f87171" dot={false} strokeWidth={1.5} name="Loss %" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Connection Count">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="connections" stroke="#a78bfa" dot={false} strokeWidth={1.5} name="Connections" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="CPU Usage (%)">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} domain={[0, 100]} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="cpu" stroke="#fbbf24" dot={false} strokeWidth={1.5} name="CPU %" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Memory Usage (GB)">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="mem_used" stroke="#f472b6" dot={false} strokeWidth={1.5} name="Used (GB)" connectNulls />
                <Line type="monotone" dataKey="mem_avail" stroke="#38bdf8" dot={false} strokeWidth={1.5} name="Available (GB)" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Load Average">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="load_1m" stroke="#34d399" dot={false} strokeWidth={1.5} name="1m" connectNulls />
                <Line type="monotone" dataKey="load_5m" stroke="#fbbf24" dot={false} strokeWidth={1.5} name="5m" connectNulls />
                <Line type="monotone" dataKey="load_15m" stroke="#f87171" dot={false} strokeWidth={1.5} name="15m" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Swap Usage (MB)">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="swap_used" stroke="#f97316" dot={false} strokeWidth={1.5} name="Swap Used (MB)" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="Disk I/O (MB)">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="disk_read" stroke="#38bdf8" dot={false} strokeWidth={1.5} name="Read (MB)" connectNulls />
                <Line type="monotone" dataKey="disk_write" stroke="#f472b6" dot={false} strokeWidth={1.5} name="Write (MB)" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>

          <ChartCard title="TCP Connection States">
            <ResponsiveContainer width="100%" height={150}>
              <LineChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#333" />
                <XAxis dataKey="time" stroke="#666" tick={{ fontSize: 11 }} interval="preserveStartEnd" />
                <YAxis stroke="#666" tick={{ fontSize: 11 }} />
                <Tooltip contentStyle={{ background: '#1a1a1a', border: '1px solid #333', fontSize: 12 }} />
                <Line type="monotone" dataKey="time_wait" stroke="#fbbf24" dot={false} strokeWidth={1.5} name="TIME_WAIT" connectNulls />
                <Line type="monotone" dataKey="close_wait" stroke="#f87171" dot={false} strokeWidth={1.5} name="CLOSE_WAIT" connectNulls />
              </LineChart>
            </ResponsiveContainer>
          </ChartCard>
        </div>
      )}
    </div>
  )
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded p-3">
      <div className="text-xs text-zinc-400">{label}</div>
      <div className="text-sm font-medium truncate">{value}</div>
    </div>
  )
}

function ChartCard({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4">
      <h3 className="text-sm font-medium text-zinc-300 mb-3">{title}</h3>
      {children}
    </div>
  )
}

function formatUptime(secs: number): string {
  const days = Math.floor(secs / 86400)
  const hours = Math.floor((secs % 86400) / 3600)
  if (days > 0) return `${days}d ${hours}h`
  const mins = Math.floor((secs % 3600) / 60)
  return `${hours}h ${mins}m`
}
