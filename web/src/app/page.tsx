'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import { useAuth } from '@/lib/auth'
import { getHosts, Host } from '@/lib/api'

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

function Landing() {
  return (
    <div className="-mx-4 -mt-6">
      {/* Hero */}
      <section className="px-4 py-20 text-center max-w-3xl mx-auto">
        <h1 className="text-4xl md:text-5xl font-bold mb-4 tracking-tight">
          Network monitoring<br />
          <span className="text-emerald-400">without the complexity</span>
        </h1>
        <p className="text-lg text-zinc-400 mb-8 max-w-xl mx-auto">
          Lightweight agent. Real-time dashboard. Instant alerts.
          Monitor your Linux fleet in under 2 minutes — no config files, no YAML, no enterprise sales calls.
        </p>
        <div className="flex gap-3 justify-center">
          <Link href="/register" className="bg-emerald-600 hover:bg-emerald-500 text-white px-6 py-2.5 rounded-lg font-medium">
            Get Started Free
          </Link>
          <a href="#how-it-works" className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-6 py-2.5 rounded-lg font-medium">
            How It Works
          </a>
        </div>
        <p className="text-xs text-zinc-500 mt-4">14-day free trial · No credit card required</p>
      </section>

      {/* Install */}
      <section className="px-4 py-8 max-w-2xl mx-auto">
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <p className="text-xs text-zinc-500 mb-2 font-medium uppercase tracking-wider">Install in one command</p>
          <div className="font-mono text-sm text-emerald-400 break-all">
            curl -sSL https://your-api/install.sh | sudo sh -s -- --api-key YOUR_KEY
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="px-4 py-16 max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-10">Everything you need. Nothing you don&apos;t.</h2>
        <div className="grid md:grid-cols-3 gap-6">
          <FeatureCard
            icon="📡"
            title="Real-Time Metrics"
            description="CPU, memory, load average, network bandwidth, connection count — collected every 15 seconds."
          />
          <FeatureCard
            icon="🏓"
            title="Health Probes"
            description="Gateway and DNS latency with packet loss detection. Know when your network degrades before users complain."
          />
          <FeatureCard
            icon="🔔"
            title="Instant Alerts"
            description="Email and Slack notifications when hosts go offline, latency spikes, or packet loss exceeds thresholds."
          />
          <FeatureCard
            icon="📊"
            title="Historical Charts"
            description="72-hour metric history with interactive charts. CPU, memory, latency, packet loss, connections, load average."
          />
          <FeatureCard
            icon="🖥️"
            title="Fleet Dashboard"
            description="All your hosts at a glance. Status, OS, CPU, memory, last seen — with automatic offline detection."
          />
          <FeatureCard
            icon="🔄"
            title="Self-Updating Agent"
            description="One command to update: sudo netwatch-agent update. Downloads the latest version and restarts automatically."
          />
        </div>
      </section>

      {/* How it works */}
      <section id="how-it-works" className="px-4 py-16 max-w-3xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-10">Up and running in 3 steps</h2>
        <div className="space-y-8">
          <Step number="1" title="Sign up" description="Create an account and get your API key. Takes 10 seconds." />
          <Step number="2" title="Install the agent" description="Run one curl command on your Linux server. The agent starts collecting metrics immediately — no config needed." />
          <Step number="3" title="Monitor" description="Open the dashboard. See your hosts, metrics, and charts in real time. Set up alerts for packet loss, latency, or host offline." />
        </div>
      </section>

      {/* What we collect */}
      <section className="px-4 py-16 max-w-3xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-10">What the agent collects</h2>
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-zinc-800">
                <th className="text-left p-3 text-zinc-400 font-medium">Metric</th>
                <th className="text-left p-3 text-zinc-400 font-medium">Source</th>
                <th className="text-left p-3 text-zinc-400 font-medium">Interval</th>
              </tr>
            </thead>
            <tbody className="text-zinc-300">
              <MetricRow metric="CPU usage (%)" source="/proc/stat" interval="15s" />
              <MetricRow metric="Memory (total, used, available)" source="/proc/meminfo" interval="15s" />
              <MetricRow metric="Load average (1m, 5m, 15m)" source="/proc/loadavg" interval="15s" />
              <MetricRow metric="Interface status & bandwidth" source="/sys/class/net/" interval="15s" />
              <MetricRow metric="Connection count" source="/proc/net/tcp" interval="15s" />
              <MetricRow metric="Gateway latency & packet loss" source="ping" interval="30s" />
              <MetricRow metric="DNS latency & packet loss" source="ping" interval="30s" />
            </tbody>
          </table>
        </div>
        <p className="text-xs text-zinc-500 mt-3 text-center">
          No packet inspection. No connection details. No sensitive data leaves your server.
        </p>
      </section>

      {/* Pricing */}
      <section className="px-4 py-16 max-w-3xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-10">Simple pricing</h2>
        <div className="grid md:grid-cols-2 gap-6 max-w-2xl mx-auto">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <h3 className="font-bold text-lg mb-1">Free Trial</h3>
            <p className="text-3xl font-bold mb-1">$0<span className="text-sm text-zinc-400 font-normal"> / 14 days</span></p>
            <ul className="text-sm text-zinc-400 space-y-2 mt-4">
              <li>✓ Up to 3 hosts</li>
              <li>✓ 24-hour data retention</li>
              <li>✓ Email alerts</li>
              <li>✓ All metrics</li>
            </ul>
          </div>
          <div className="bg-zinc-900 border border-emerald-800 rounded-xl p-6 relative">
            <div className="absolute -top-3 left-4 bg-emerald-600 text-white text-xs px-2 py-0.5 rounded-full font-medium">Early Access</div>
            <h3 className="font-bold text-lg mb-1">Pro</h3>
            <p className="text-3xl font-bold mb-1">$49<span className="text-sm text-zinc-400 font-normal"> / month</span></p>
            <ul className="text-sm text-zinc-400 space-y-2 mt-4">
              <li>✓ Up to 10 hosts</li>
              <li>✓ 72-hour data retention</li>
              <li>✓ Email + Slack alerts</li>
              <li>✓ All metrics</li>
              <li>✓ Direct founder support</li>
            </ul>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="px-4 py-16 text-center">
        <h2 className="text-2xl font-bold mb-4">Start monitoring in 2 minutes</h2>
        <p className="text-zinc-400 mb-6">No credit card. No sales calls. Just sign up and install.</p>
        <Link href="/register" className="bg-emerald-600 hover:bg-emerald-500 text-white px-8 py-3 rounded-lg font-medium text-lg">
          Get Started Free
        </Link>
      </section>

      {/* Footer */}
      <footer className="border-t border-zinc-800 px-4 py-8 text-center text-xs text-zinc-500">
        <p>NetWatch Cloud — lightweight network monitoring for Linux fleets</p>
        <p className="mt-1">Built with Rust + Next.js · Powered by Railway</p>
      </footer>
    </div>
  )
}

function FeatureCard({ icon, title, description }: { icon: string; title: string; description: string }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
      <div className="text-2xl mb-2">{icon}</div>
      <h3 className="font-semibold mb-1">{title}</h3>
      <p className="text-sm text-zinc-400">{description}</p>
    </div>
  )
}

function Step({ number, title, description }: { number: string; title: string; description: string }) {
  return (
    <div className="flex gap-4">
      <div className="w-8 h-8 rounded-full bg-emerald-600 flex items-center justify-center text-sm font-bold shrink-0">{number}</div>
      <div>
        <h3 className="font-semibold mb-1">{title}</h3>
        <p className="text-sm text-zinc-400">{description}</p>
      </div>
    </div>
  )
}

function MetricRow({ metric, source, interval }: { metric: string; source: string; interval: string }) {
  return (
    <tr className="border-b border-zinc-800/50">
      <td className="p-3">{metric}</td>
      <td className="p-3 font-mono text-xs text-zinc-500">{source}</td>
      <td className="p-3 text-zinc-500">{interval}</td>
    </tr>
  )
}

export default function HostsPage() {
  const { token, isLoading: authLoading } = useAuth()
  const [hosts, setHosts] = useState<Host[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (authLoading || !token) return

    async function fetch() {
      try {
        const data = await getHosts()
        setHosts(data)
      } catch {
        // handled by api client redirect
      } finally {
        setLoading(false)
      }
    }

    fetch()
    const interval = setInterval(fetch, 60_000)
    return () => clearInterval(interval)
  }, [token, authLoading])

  if (authLoading) return null

  if (!token) {
    return <Landing />
  }

  if (loading) {
    return <div className="text-zinc-400 mt-10">Loading hosts...</div>
  }

  if (hosts.length === 0) {
    return (
      <div className="mt-10">
        <h1 className="text-2xl font-bold mb-4">No hosts connected</h1>
        <p className="text-zinc-400 mb-4">Install the NetWatch agent on a Linux server to get started.</p>
        <p className="text-zinc-400 text-sm">Go to <Link href="/settings" className="text-emerald-400 hover:underline">Settings</Link> to get your API key and install command.</p>
      </div>
    )
  }

  return (
    <div>
      <h1 className="text-2xl font-bold mb-6">Hosts</h1>
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
        {hosts.map(host => (
          <Link
            key={host.id}
            href={`/hosts/${host.id}`}
            className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 hover:border-zinc-600 transition-colors"
          >
            <div className="flex items-center gap-2 mb-2">
              <span className={`w-2.5 h-2.5 rounded-full ${host.is_online ? 'bg-emerald-400' : 'bg-red-400'}`} />
              <span className="font-semibold">{host.hostname}</span>
            </div>
            <div className="text-xs text-zinc-400 space-y-1">
              {host.os && <div>{host.os}</div>}
              <div>Last seen: {new Date(host.last_seen_at).toLocaleString()}</div>
              {host.agent_version && <div>Agent v{host.agent_version}</div>}
              {(host.cpu_cores || host.memory_total_bytes) && (
                <div className="flex gap-2">
                  {host.cpu_cores && <span>{host.cpu_cores} cores</span>}
                  {host.memory_total_bytes && <span>{formatBytes(host.memory_total_bytes)} RAM</span>}
                </div>
              )}
            </div>
          </Link>
        ))}
      </div>
    </div>
  )
}
