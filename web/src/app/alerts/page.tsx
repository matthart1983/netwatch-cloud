'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import {
  getAlertRules, createAlertRule, updateAlertRule, deleteAlertRule,
  getAlertHistory, AlertRule, AlertEvent
} from '@/lib/api'

const METRICS = [
  { value: 'host_status', label: 'Host Status', type: 'status' },
  { value: 'interface_status', label: 'Interface Status', type: 'status' },
  { value: 'gateway_rtt_ms', label: 'Gateway Latency (ms)', type: 'numeric' },
  { value: 'gateway_loss_pct', label: 'Gateway Packet Loss (%)', type: 'numeric' },
  { value: 'dns_rtt_ms', label: 'DNS Latency (ms)', type: 'numeric' },
  { value: 'dns_loss_pct', label: 'DNS Packet Loss (%)', type: 'numeric' },
  { value: 'connection_count', label: 'Connection Count', type: 'numeric' },
]

export default function AlertsPage() {
  const { token, isLoading: authLoading } = useAuth()
  const router = useRouter()
  const [rules, setRules] = useState<AlertRule[]>([])
  const [events, setEvents] = useState<AlertEvent[]>([])
  const [tab, setTab] = useState<'rules' | 'history'>('rules')
  const [showForm, setShowForm] = useState(false)
  const [loading, setLoading] = useState(true)

  // Form state
  const [formName, setFormName] = useState('')
  const [formMetric, setFormMetric] = useState('gateway_loss_pct')
  const [formCondition, setFormCondition] = useState('>')
  const [formThreshold, setFormThreshold] = useState('5')
  const [formDuration, setFormDuration] = useState('60')
  const [formSeverity, setFormSeverity] = useState('warning')

  useEffect(() => {
    if (authLoading) return
    if (!token) { router.push('/login'); return }
    loadData()
  }, [authLoading, token, router])

  async function loadData() {
    try {
      const [r, e] = await Promise.all([getAlertRules(), getAlertHistory()])
      setRules(r)
      setEvents(e)
    } catch {} finally {
      setLoading(false)
    }
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault()
    const metricDef = METRICS.find(m => m.value === formMetric)
    const isStatus = metricDef?.type === 'status'

    await createAlertRule({
      name: formName || `${metricDef?.label} alert`,
      metric: formMetric,
      condition: isStatus ? 'changes_to' : formCondition,
      threshold: isStatus ? undefined : parseFloat(formThreshold),
      threshold_str: isStatus ? (formMetric === 'host_status' ? 'offline' : 'down') : undefined,
      duration_secs: parseInt(formDuration),
      severity: formSeverity,
    })
    setShowForm(false)
    setFormName('')
    loadData()
  }

  async function handleToggle(rule: AlertRule) {
    await updateAlertRule(rule.id, { enabled: !rule.enabled })
    loadData()
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this alert rule?')) return
    await deleteAlertRule(id)
    loadData()
  }

  if (authLoading || loading) return <div className="text-zinc-400 mt-10">Loading...</div>

  return (
    <div className="max-w-3xl">
      <h1 className="text-2xl font-bold mb-6">Alerts</h1>

      <div className="flex gap-2 mb-6">
        <button
          onClick={() => setTab('rules')}
          className={`px-4 py-1.5 rounded text-sm ${tab === 'rules' ? 'bg-emerald-600 text-white' : 'bg-zinc-800 text-zinc-400'}`}
        >
          Rules ({rules.length})
        </button>
        <button
          onClick={() => setTab('history')}
          className={`px-4 py-1.5 rounded text-sm ${tab === 'history' ? 'bg-emerald-600 text-white' : 'bg-zinc-800 text-zinc-400'}`}
        >
          History ({events.length})
        </button>
      </div>

      {tab === 'rules' && (
        <div>
          <div className="space-y-2 mb-4">
            {rules.map(rule => (
              <div key={rule.id} className="bg-zinc-900 border border-zinc-800 rounded p-4 flex items-center justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <span className={`text-xs px-1.5 py-0.5 rounded ${
                      rule.severity === 'critical' ? 'bg-red-900 text-red-300' :
                      rule.severity === 'warning' ? 'bg-yellow-900 text-yellow-300' :
                      'bg-blue-900 text-blue-300'
                    }`}>
                      {rule.severity}
                    </span>
                    <span className={`font-medium ${rule.enabled ? 'text-zinc-100' : 'text-zinc-500'}`}>
                      {rule.name}
                    </span>
                  </div>
                  <div className="text-xs text-zinc-500 mt-1">
                    {rule.metric} {rule.condition} {rule.threshold ?? rule.threshold_str} · {rule.duration_secs}s · {rule.host_id ? 'specific host' : 'all hosts'}
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <button
                    onClick={() => handleToggle(rule)}
                    className={`text-xs px-2 py-1 rounded ${rule.enabled ? 'bg-emerald-900 text-emerald-300' : 'bg-zinc-700 text-zinc-400'}`}
                  >
                    {rule.enabled ? 'Enabled' : 'Disabled'}
                  </button>
                  <button onClick={() => handleDelete(rule.id)} className="text-red-400 hover:text-red-300 text-xs">
                    Delete
                  </button>
                </div>
              </div>
            ))}
          </div>

          {showForm ? (
            <form onSubmit={handleCreate} className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-3">
              <div>
                <label className="block text-xs text-zinc-400 mb-1">Name</label>
                <input value={formName} onChange={e => setFormName(e.target.value)} placeholder="e.g. High latency alert" className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm" />
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Metric</label>
                  <select value={formMetric} onChange={e => setFormMetric(e.target.value)} className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm">
                    {METRICS.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
                  </select>
                </div>
                {METRICS.find(m => m.value === formMetric)?.type === 'numeric' && (
                  <div>
                    <label className="block text-xs text-zinc-400 mb-1">Threshold</label>
                    <div className="flex gap-2">
                      <select value={formCondition} onChange={e => setFormCondition(e.target.value)} className="bg-zinc-800 border border-zinc-700 rounded px-2 py-1.5 text-sm">
                        <option value=">">&gt;</option>
                        <option value="<">&lt;</option>
                      </select>
                      <input type="number" step="any" value={formThreshold} onChange={e => setFormThreshold(e.target.value)} className="flex-1 bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm" />
                    </div>
                  </div>
                )}
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Duration (seconds)</label>
                  <input type="number" value={formDuration} onChange={e => setFormDuration(e.target.value)} className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm" />
                </div>
                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Severity</label>
                  <select value={formSeverity} onChange={e => setFormSeverity(e.target.value)} className="w-full bg-zinc-800 border border-zinc-700 rounded px-3 py-1.5 text-sm">
                    <option value="info">Info</option>
                    <option value="warning">Warning</option>
                    <option value="critical">Critical</option>
                  </select>
                </div>
              </div>
              <div className="flex gap-2">
                <button type="submit" className="bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-1.5 rounded text-sm">Create Rule</button>
                <button type="button" onClick={() => setShowForm(false)} className="text-zinc-400 hover:text-zinc-100 text-sm">Cancel</button>
              </div>
            </form>
          ) : (
            <button onClick={() => setShowForm(true)} className="bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded text-sm">
              New Alert Rule
            </button>
          )}
        </div>
      )}

      {tab === 'history' && (
        <div className="space-y-2">
          {events.length === 0 ? (
            <p className="text-zinc-400">No alert events yet.</p>
          ) : (
            events.map(event => (
              <div key={event.id} className="bg-zinc-900 border border-zinc-800 rounded p-3 flex items-center gap-3">
                <span className={`w-2 h-2 rounded-full ${event.state === 'firing' ? 'bg-red-400' : 'bg-emerald-400'}`} />
                <div className="flex-1">
                  <div className="text-sm">{event.message}</div>
                  <div className="text-xs text-zinc-500">{new Date(event.created_at).toLocaleString()}</div>
                </div>
                <span className={`text-xs px-1.5 py-0.5 rounded ${event.state === 'firing' ? 'bg-red-900 text-red-300' : 'bg-emerald-900 text-emerald-300'}`}>
                  {event.state}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  )
}
