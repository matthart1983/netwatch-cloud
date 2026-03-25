'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import { useAuth } from '@/lib/auth'
import { getHosts, Host } from '@/lib/api'
import {
  Activity, Radar, Bell, BarChart3, Monitor, RefreshCw,
  Shield, Lock, Eye, ChevronRight, Zap, X, Check
} from 'lucide-react'

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

function LandingNav() {
  return (
    <nav className="border-b border-zinc-800 bg-zinc-950/80 backdrop-blur-sm sticky top-0 z-50">
      <div className="max-w-6xl mx-auto px-4 h-14 flex items-center justify-between">
        <span className="text-lg font-bold text-emerald-400">NetWatch</span>
        <div className="hidden md:flex items-center gap-6">
          <a href="#features" className="text-sm text-zinc-400 hover:text-zinc-100 transition-colors">Features</a>
          <a href="#how-it-works" className="text-sm text-zinc-400 hover:text-zinc-100 transition-colors">How It Works</a>
          <a href="#pricing" className="text-sm text-zinc-400 hover:text-zinc-100 transition-colors">Pricing</a>
          <a href="#security" className="text-sm text-zinc-400 hover:text-zinc-100 transition-colors">Security</a>
        </div>
        <div className="flex items-center gap-3">
          <Link href="/login" className="text-sm text-zinc-400 hover:text-zinc-100 transition-colors">Login</Link>
          <Link href="/register" className="text-sm bg-emerald-600 hover:bg-emerald-500 text-white px-3 py-1.5 rounded transition-colors">
            Sign Up
          </Link>
        </div>
      </div>
    </nav>
  )
}

function DashboardMockup() {
  const mockHosts = [
    { name: 'web-prod-1', os: 'Ubuntu 24.04', online: true, cpu: 23, mem: '7.2 GB', cores: 4 },
    { name: 'api-prod-1', os: 'Debian 12', online: true, cpu: 45, mem: '15.8 GB', cores: 8 },
    { name: 'db-replica-2', os: 'Ubuntu 22.04', online: false, cpu: 0, mem: '31.4 GB', cores: 16 },
  ]

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden shadow-2xl shadow-emerald-900/10">
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-zinc-800 bg-zinc-900/50">
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
        </div>
        <span className="text-xs text-zinc-500 ml-2">netwatch-web-production.up.railway.app</span>
      </div>
      <div className="p-4">
        <div className="text-sm font-semibold mb-3 text-zinc-300">Hosts</div>
        <div className="grid gap-2">
          {mockHosts.map(host => (
            <div key={host.name} className="bg-zinc-800/50 border border-zinc-700/50 rounded-lg p-3 flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <span className={`w-2 h-2 rounded-full ${host.online ? 'bg-emerald-400' : 'bg-red-400'}`} />
                <div>
                  <div className="text-sm font-medium text-zinc-200">{host.name}</div>
                  <div className="text-xs text-zinc-500">{host.os}</div>
                </div>
              </div>
              <div className="flex items-center gap-4 text-xs text-zinc-400">
                {host.online && <span>CPU {host.cpu}%</span>}
                <span>{host.cores} cores</span>
                <span>{host.mem} RAM</span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}

function ChartMockup() {
  const points = [2.1, 1.8, 2.4, 1.5, 3.2, 8.7, 12.4, 9.1, 3.8, 2.2, 1.9, 2.0, 1.7, 2.3, 1.8]
  const max = Math.max(...points)
  const width = 400
  const height = 120
  const padding = 20

  const pathData = points
    .map((p, i) => {
      const x = padding + (i / (points.length - 1)) * (width - padding * 2)
      const y = height - padding - (p / max) * (height - padding * 2)
      return `${i === 0 ? 'M' : 'L'} ${x} ${y}`
    })
    .join(' ')

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden shadow-2xl shadow-emerald-900/10">
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-zinc-800 bg-zinc-900/50">
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
        </div>
        <span className="text-xs text-zinc-500 ml-2">Host Detail — web-prod-1</span>
      </div>
      <div className="p-4">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm font-semibold text-zinc-300">Gateway Latency (ms)</span>
          <div className="flex gap-1">
            {['1h', '6h', '24h', '72h'].map(r => (
              <span key={r} className={`text-xs px-2 py-0.5 rounded ${r === '24h' ? 'bg-emerald-600 text-white' : 'bg-zinc-800 text-zinc-500'}`}>{r}</span>
            ))}
          </div>
        </div>
        <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-auto">
          <path d={pathData} fill="none" stroke="#34d399" strokeWidth="2" strokeLinejoin="round" />
          <circle cx={padding + (6 / (points.length - 1)) * (width - padding * 2)} cy={height - padding - (12.4 / max) * (height - padding * 2)} r="4" fill="#34d399" />
          <rect x={padding + (6 / (points.length - 1)) * (width - padding * 2) - 30} y={height - padding - (12.4 / max) * (height - padding * 2) - 24} width="60" height="18" rx="4" fill="#27272a" stroke="#3f3f46" strokeWidth="1" />
          <text x={padding + (6 / (points.length - 1)) * (width - padding * 2)} y={height - padding - (12.4 / max) * (height - padding * 2) - 12} textAnchor="middle" fill="#a1a1aa" fontSize="10">12.4 ms</text>
        </svg>
      </div>
    </div>
  )
}

function AlertMockup() {
  const events = [
    { state: 'firing', message: 'CRITICAL: Host offline on db-replica-2', time: '2 min ago' },
    { state: 'resolved', message: 'RESOLVED: Gateway latency on web-prod-1', time: '14 min ago' },
    { state: 'firing', message: 'WARNING: Packet loss > 5% on api-prod-1', time: '23 min ago' },
  ]

  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-hidden shadow-2xl shadow-emerald-900/10">
      <div className="flex items-center gap-2 px-4 py-2.5 border-b border-zinc-800 bg-zinc-900/50">
        <div className="flex gap-1.5">
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
          <div className="w-3 h-3 rounded-full bg-zinc-700" />
        </div>
        <span className="text-xs text-zinc-500 ml-2">Alerts — History</span>
      </div>
      <div className="p-4 space-y-2">
        {events.map((e, i) => (
          <div key={i} className="bg-zinc-800/50 border border-zinc-700/50 rounded-lg p-3 flex items-center gap-3">
            <span className={`w-2 h-2 rounded-full shrink-0 ${e.state === 'firing' ? 'bg-red-400' : 'bg-emerald-400'}`} />
            <div className="flex-1 min-w-0">
              <div className="text-sm text-zinc-200 truncate">{e.message}</div>
            </div>
            <span className="text-xs text-zinc-500 shrink-0">{e.time}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

function Landing() {
  return (
    <div className="-mx-4 -mt-6">
      <LandingNav />

      {/* Hero */}
      <section className="px-4 pt-20 pb-12 text-center max-w-3xl mx-auto">
        <div className="inline-flex items-center gap-2 bg-emerald-950/50 border border-emerald-800/50 text-emerald-400 text-xs px-3 py-1 rounded-full mb-6">
          <Zap className="w-3 h-3" />
          Now in Early Access
        </div>
        <h1 className="text-4xl md:text-5xl font-bold mb-4 tracking-tight">
          Network monitoring<br />
          <span className="text-emerald-400">without the complexity</span>
        </h1>
        <p className="text-lg text-zinc-400 mb-8 max-w-xl mx-auto">
          Lightweight agent. Real-time dashboard. Instant alerts.
          Monitor your Linux fleet in under 2 minutes — no config files, no YAML, no enterprise sales calls.
        </p>
        <div className="flex gap-3 justify-center flex-wrap">
          <Link href="/register" className="bg-emerald-600 hover:bg-emerald-500 text-white px-6 py-2.5 rounded-lg font-medium transition-colors inline-flex items-center gap-2">
            Get Started Free <ChevronRight className="w-4 h-4" />
          </Link>
          <a href="#how-it-works" className="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-6 py-2.5 rounded-lg font-medium transition-colors">
            How It Works
          </a>
        </div>
        <p className="text-xs text-zinc-500 mt-4">14-day free trial · No credit card required</p>
      </section>

      {/* Dashboard screenshot mockup */}
      <section className="px-4 pb-16 max-w-4xl mx-auto">
        <DashboardMockup />
      </section>

      {/* Install */}
      <section className="px-4 py-8 max-w-2xl mx-auto">
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <p className="text-xs text-zinc-500 mb-2 font-medium uppercase tracking-wider">Install in one command</p>
          <div className="font-mono text-sm text-emerald-400 break-all">
            curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sh -s -- --api-key YOUR_KEY
          </div>
          <p className="text-xs text-zinc-600 mt-3">
            <a href="https://netwatch-api-production.up.railway.app/install.sh" target="_blank" rel="noopener noreferrer" className="hover:text-zinc-400 underline underline-offset-2">
              View the install script source ↗
            </a>
          </p>
        </div>
      </section>

      {/* Features */}
      <section id="features" className="px-4 py-16 max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-3">Everything you need. Nothing you don&apos;t.</h2>
        <p className="text-zinc-500 text-center mb-10 max-w-lg mx-auto">Purpose-built for Linux infrastructure. No bloated agent, no complex config, no hidden costs.</p>
        <div className="grid md:grid-cols-3 gap-6">
          <FeatureCard
            icon={<Activity className="w-5 h-5 text-emerald-400" />}
            title="Real-Time Metrics"
            description="CPU, memory, load average, network bandwidth, connection count — collected every 15 seconds."
          />
          <FeatureCard
            icon={<Radar className="w-5 h-5 text-emerald-400" />}
            title="Health Probes"
            description="Gateway and DNS latency with packet loss detection. Know when your network degrades before users complain."
          />
          <FeatureCard
            icon={<Bell className="w-5 h-5 text-emerald-400" />}
            title="Instant Alerts"
            description="Email and Slack notifications when hosts go offline, latency spikes, or packet loss exceeds thresholds."
          />
          <FeatureCard
            icon={<BarChart3 className="w-5 h-5 text-emerald-400" />}
            title="Historical Charts"
            description="72-hour metric history with interactive charts. CPU, memory, latency, packet loss, connections, load average."
          />
          <FeatureCard
            icon={<Monitor className="w-5 h-5 text-emerald-400" />}
            title="Fleet Dashboard"
            description="All your hosts at a glance. Status, OS, CPU, memory, last seen — with automatic offline detection."
          />
          <FeatureCard
            icon={<RefreshCw className="w-5 h-5 text-emerald-400" />}
            title="Self-Updating Agent"
            description="One command to update. Downloads the latest version and restarts automatically. No manual work."
          />
        </div>
      </section>

      {/* Product screenshots */}
      <section className="px-4 py-8 max-w-5xl mx-auto">
        <div className="grid md:grid-cols-2 gap-6">
          <ChartMockup />
          <AlertMockup />
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
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-x-auto">
          <table className="w-full text-sm min-w-[480px]">
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

      {/* Why NetWatch vs alternatives */}
      <section className="px-4 py-16 max-w-4xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-3">Why NetWatch?</h2>
        <p className="text-zinc-500 text-center mb-10 max-w-lg mx-auto">You have options. Here&apos;s how we compare.</p>
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl overflow-x-auto">
          <table className="w-full text-sm min-w-[600px]">
            <thead>
              <tr className="border-b border-zinc-800">
                <th className="text-left p-3 text-zinc-400 font-medium" />
                <th className="text-left p-3 text-emerald-400 font-semibold">NetWatch</th>
                <th className="text-left p-3 text-zinc-400 font-medium">Datadog</th>
                <th className="text-left p-3 text-zinc-400 font-medium">Uptime Kuma</th>
                <th className="text-left p-3 text-zinc-400 font-medium">PRTG</th>
              </tr>
            </thead>
            <tbody className="text-zinc-300">
              <tr className="border-b border-zinc-800/50">
                <td className="p-3 text-zinc-400">Setup time</td>
                <td className="p-3"><span className="text-emerald-400 font-medium">2 minutes</span></td>
                <td className="p-3">30+ minutes</td>
                <td className="p-3">15+ minutes</td>
                <td className="p-3">1+ hours</td>
              </tr>
              <tr className="border-b border-zinc-800/50">
                <td className="p-3 text-zinc-400">Agent footprint</td>
                <td className="p-3"><span className="text-emerald-400 font-medium">~5 MB single binary</span></td>
                <td className="p-3">~800 MB</td>
                <td className="p-3">No agent (external)</td>
                <td className="p-3">~200 MB</td>
              </tr>
              <tr className="border-b border-zinc-800/50">
                <td className="p-3 text-zinc-400">Requires root</td>
                <td className="p-3"><X className="w-4 h-4 text-emerald-400 inline" /> No</td>
                <td className="p-3"><Check className="w-4 h-4 text-zinc-500 inline" /> Yes</td>
                <td className="p-3">N/A</td>
                <td className="p-3"><Check className="w-4 h-4 text-zinc-500 inline" /> Yes</td>
              </tr>
              <tr className="border-b border-zinc-800/50">
                <td className="p-3 text-zinc-400">Self-hosted required</td>
                <td className="p-3"><X className="w-4 h-4 text-emerald-400 inline" /> No</td>
                <td className="p-3"><X className="w-4 h-4 text-zinc-500 inline" /> No</td>
                <td className="p-3"><Check className="w-4 h-4 text-zinc-500 inline" /> Yes</td>
                <td className="p-3"><Check className="w-4 h-4 text-zinc-500 inline" /> Yes</td>
              </tr>
              <tr className="border-b border-zinc-800/50">
                <td className="p-3 text-zinc-400">Config files</td>
                <td className="p-3"><span className="text-emerald-400 font-medium">Zero</span></td>
                <td className="p-3">YAML</td>
                <td className="p-3">Web UI</td>
                <td className="p-3">Extensive</td>
              </tr>
              <tr>
                <td className="p-3 text-zinc-400">Starting price</td>
                <td className="p-3"><span className="text-emerald-400 font-medium">$49/mo flat</span></td>
                <td className="p-3">$15/host/mo+</td>
                <td className="p-3">Free (self-host)</td>
                <td className="p-3">$1,750+/yr</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p className="text-xs text-zinc-500 mt-4 text-center max-w-lg mx-auto">
          Uptime Kuma is great if you want to self-host and only need external pings.
          Datadog is great if you need 500+ integrations. NetWatch is for teams that want host-level network
          monitoring that just works, with zero ops overhead.
        </p>
      </section>

      {/* Security */}
      <section id="security" className="px-4 py-16 max-w-3xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-3">Security & Privacy</h2>
        <p className="text-zinc-500 text-center mb-10 max-w-lg mx-auto">Your infrastructure data is sensitive. Here&apos;s how we treat it.</p>
        <div className="grid md:grid-cols-3 gap-6">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
            <Lock className="w-5 h-5 text-emerald-400 mb-3" />
            <h3 className="font-semibold mb-1">Encrypted in transit</h3>
            <p className="text-sm text-zinc-400">All agent→API communication uses HTTPS/TLS. API keys are bcrypt-hashed and never stored in plaintext.</p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
            <Eye className="w-5 h-5 text-emerald-400 mb-3" />
            <h3 className="font-semibold mb-1">No packet inspection</h3>
            <p className="text-sm text-zinc-400">The agent reads counters from /proc and /sys. It never captures packet contents, connection IPs, or process names.</p>
          </div>
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
            <Shield className="w-5 h-5 text-emerald-400 mb-3" />
            <h3 className="font-semibold mb-1">No root required</h3>
            <p className="text-sm text-zinc-400">The agent runs as an unprivileged user. All 9 metric sources are readable without elevated permissions on Linux.</p>
          </div>
        </div>
        <div className="mt-6 text-center">
          <a
            href="https://netwatch-api-production.up.railway.app/install.sh"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-zinc-400 hover:text-emerald-400 underline underline-offset-2 transition-colors"
          >
            Audit the install script source code ↗
          </a>
        </div>
      </section>

      {/* Pricing */}
      <section id="pricing" className="px-4 py-16 max-w-3xl mx-auto">
        <h2 className="text-2xl font-bold text-center mb-3">Simple pricing</h2>
        <p className="text-zinc-500 text-center mb-10">No per-host fees. No surprise overages. One price.</p>
        <div className="grid md:grid-cols-2 gap-6 max-w-2xl mx-auto">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <h3 className="font-bold text-lg mb-1">Free Trial</h3>
            <p className="text-3xl font-bold mb-1">$0<span className="text-sm text-zinc-400 font-normal"> / 14 days</span></p>
            <ul className="text-sm text-zinc-400 space-y-2 mt-4">
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-zinc-500" /> Up to 3 hosts</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-zinc-500" /> 24-hour data retention</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-zinc-500" /> Email alerts</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-zinc-500" /> All metrics</li>
            </ul>
            <Link href="/register" className="block text-center mt-6 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 px-4 py-2 rounded-lg text-sm font-medium transition-colors">
              Start Free Trial
            </Link>
          </div>
          <div className="bg-zinc-900 border border-emerald-800/60 rounded-xl p-6 relative">
            <div className="absolute -top-3 left-4 bg-emerald-600 text-white text-xs px-2.5 py-0.5 rounded-full font-medium">Early Access</div>
            <h3 className="font-bold text-lg mb-1">Pro</h3>
            <p className="text-3xl font-bold mb-1">$49<span className="text-sm text-zinc-400 font-normal"> / month</span></p>
            <ul className="text-sm text-zinc-400 space-y-2 mt-4">
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-emerald-400" /> Up to 10 hosts</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-emerald-400" /> 72-hour data retention</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-emerald-400" /> Email + Slack alerts</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-emerald-400" /> All metrics</li>
              <li className="flex items-center gap-2"><Check className="w-4 h-4 text-emerald-400" /> Direct founder support</li>
            </ul>
            <Link href="/register" className="block text-center mt-6 bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded-lg text-sm font-medium transition-colors">
              Get Started
            </Link>
          </div>
        </div>
      </section>

      {/* Built by */}
      <section className="px-4 py-12 max-w-xl mx-auto text-center">
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <div className="w-12 h-12 rounded-full bg-emerald-600 flex items-center justify-center text-lg font-bold mx-auto mb-3">M</div>
          <p className="text-zinc-300 text-sm mb-1">
            Built by <strong>Matt</strong> — a solo founder who got tired of configuring Nagios and paying $15/host/month for Datadog just to check if his servers can reach the internet.
          </p>
          <p className="text-xs text-zinc-500 mt-2">
            NetWatch is built with Rust (agent + API) and Next.js (dashboard). The agent binary is ~5 MB and uses zero dependencies at runtime.
          </p>
        </div>
      </section>

      {/* CTA */}
      <section className="px-4 py-16 text-center">
        <h2 className="text-2xl font-bold mb-4">Start monitoring in 2 minutes</h2>
        <p className="text-zinc-400 mb-6">No credit card. No sales calls. Just sign up and install.</p>
        <Link href="/register" className="bg-emerald-600 hover:bg-emerald-500 text-white px-8 py-3 rounded-lg font-medium text-lg transition-colors inline-flex items-center gap-2">
          Get Started Free <ChevronRight className="w-5 h-5" />
        </Link>
      </section>

      {/* Footer */}
      <footer className="border-t border-zinc-800 px-4 py-8 text-center text-xs text-zinc-500">
        <p>NetWatch Cloud — lightweight network monitoring for Linux fleets</p>
        <p className="mt-1">Built with Rust + Next.js</p>
      </footer>
    </div>
  )
}

function FeatureCard({ icon, title, description }: { icon: React.ReactNode; title: string; description: string }) {
  return (
    <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-5">
      <div className="mb-3">{icon}</div>
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
