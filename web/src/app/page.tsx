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
    return (
      <div className="mt-20 text-center">
        <h1 className="text-3xl font-bold mb-4">NetWatch Cloud</h1>
        <p className="text-zinc-400 mb-6">Network monitoring for your fleet</p>
        <Link href="/register" className="bg-emerald-600 hover:bg-emerald-500 text-white px-6 py-2 rounded">
          Get Started
        </Link>
      </div>
    )
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
