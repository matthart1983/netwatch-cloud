'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import { getApiKeys, createApiKey, deleteApiKey, ApiKeyInfo } from '@/lib/api'

export default function SettingsPage() {
  const { token, isLoading: authLoading } = useAuth()
  const router = useRouter()
  const [keys, setKeys] = useState<ApiKeyInfo[]>([])
  const [newKey, setNewKey] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (authLoading) return
    if (!token) { router.push('/login'); return }
    loadKeys()
  }, [authLoading, token, router])

  async function loadKeys() {
    try {
      const data = await getApiKeys()
      setKeys(data)
    } catch {
      // handled
    } finally {
      setLoading(false)
    }
  }

  async function handleCreate() {
    try {
      const result = await createApiKey()
      setNewKey(result.api_key)
      loadKeys()
    } catch {
      // handled
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Revoke this API key? Agents using it will stop sending data.')) return
    try {
      await deleteApiKey(id)
      loadKeys()
    } catch {
      // handled
    }
  }

  if (authLoading || loading) return <div className="text-zinc-400 mt-10">Loading...</div>

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      <section className="mb-8">
        <h2 className="text-lg font-semibold mb-4">API Keys</h2>

        {newKey && (
          <div className="bg-zinc-900 border border-emerald-700 rounded-lg p-4 mb-4">
            <p className="text-sm text-emerald-400 mb-2">New API key created (shown once):</p>
            <div className="font-mono text-sm break-all mb-2">{newKey}</div>
            <div className="flex gap-2">
              <button
                onClick={() => navigator.clipboard.writeText(newKey)}
                className="bg-zinc-700 hover:bg-zinc-600 text-white px-3 py-1 rounded text-xs"
              >
                Copy
              </button>
              <button
                onClick={() => setNewKey(null)}
                className="text-zinc-400 hover:text-zinc-100 text-xs"
              >
                Dismiss
              </button>
            </div>
          </div>
        )}

        <div className="space-y-2 mb-4">
          {keys.map(key => (
            <div key={key.id} className="bg-zinc-900 border border-zinc-800 rounded p-3 flex items-center justify-between">
              <div>
                <span className="font-mono text-sm">{key.key_prefix}...</span>
                {key.label && <span className="text-zinc-400 text-xs ml-2">({key.label})</span>}
                <div className="text-xs text-zinc-500 mt-1">
                  Created {new Date(key.created_at).toLocaleDateString()}
                  {key.last_used_at && ` · Last used ${new Date(key.last_used_at).toLocaleDateString()}`}
                </div>
              </div>
              <button
                onClick={() => handleDelete(key.id)}
                className="text-red-400 hover:text-red-300 text-xs"
              >
                Revoke
              </button>
            </div>
          ))}
        </div>

        <button
          onClick={handleCreate}
          className="bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded text-sm"
        >
          Create New API Key
        </button>
      </section>

      <section>
        <h2 className="text-lg font-semibold mb-4">Install Agent</h2>
        <p className="text-zinc-400 text-sm mb-2">Run this on your Linux server:</p>
        <div className="bg-zinc-900 border border-zinc-800 rounded p-3 font-mono text-xs break-all">
          curl -sSL https://install.netwatch.dev | sh -s -- --api-key YOUR_API_KEY
        </div>
      </section>
    </div>
  )
}
