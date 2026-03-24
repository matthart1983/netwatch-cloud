'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import Link from 'next/link'
import { register as apiRegister, login as apiLogin } from '@/lib/api'
import { useAuth } from '@/lib/auth'

export default function RegisterPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [loading, setLoading] = useState(false)
  const router = useRouter()
  const { login } = useAuth()

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    setError('')
    setLoading(true)
    try {
      const reg = await apiRegister(email, password)
      setApiKey(reg.api_key)
      // Auto-login
      const data = await apiLogin(email, password)
      login(data.token, data.account_id)
    } catch {
      setError('Registration failed. Email may already be in use.')
    } finally {
      setLoading(false)
    }
  }

  if (apiKey) {
    return (
      <div className="max-w-lg mx-auto mt-20">
        <h1 className="text-2xl font-bold mb-4 text-emerald-400">Account Created!</h1>
        <p className="text-zinc-300 mb-4">Your API key (shown once — copy it now):</p>
        <div className="bg-zinc-900 border border-zinc-700 rounded p-3 mb-4 font-mono text-sm break-all">
          {apiKey}
        </div>
        <button
          onClick={() => navigator.clipboard.writeText(apiKey)}
          className="bg-zinc-700 hover:bg-zinc-600 text-white px-4 py-2 rounded text-sm mr-3"
        >
          Copy Key
        </button>
        <p className="text-zinc-400 text-sm mt-4 mb-2">Install the agent on your Linux server:</p>
        <div className="bg-zinc-900 border border-zinc-700 rounded p-3 font-mono text-xs break-all">
          curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sudo sh -s -- --api-key {apiKey} --endpoint https://netwatch-api-production.up.railway.app/api/v1/ingest
        </div>
        <p className="text-zinc-400 text-sm mt-4 mb-2">After install, manage with:</p>
        <div className="bg-zinc-900 border border-zinc-700 rounded p-3 font-mono text-xs space-y-1">
          <div><span className="text-zinc-500"># Check status</span></div>
          <div>netwatch-agent status</div>
          <div className="pt-1"><span className="text-zinc-500"># Update to latest version</span></div>
          <div>sudo netwatch-agent update</div>
          <div className="pt-1"><span className="text-zinc-500"># View logs</span></div>
          <div>journalctl -u netwatch-agent -f</div>
        </div>
        <button
          onClick={() => router.push('/')}
          className="mt-6 bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded text-sm"
        >
          Go to Dashboard
        </button>
      </div>
    )
  }

  return (
    <div className="max-w-sm mx-auto mt-20">
      <h1 className="text-2xl font-bold mb-6">Create Account</h1>
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && <p className="text-red-400 text-sm">{error}</p>}
        <div>
          <label className="block text-sm text-zinc-400 mb-1">Email</label>
          <input
            type="email"
            value={email}
            onChange={e => setEmail(e.target.value)}
            className="w-full bg-zinc-900 border border-zinc-700 rounded px-3 py-2 text-sm focus:outline-none focus:border-emerald-500"
            required
          />
        </div>
        <div>
          <label className="block text-sm text-zinc-400 mb-1">Password (8+ characters)</label>
          <input
            type="password"
            value={password}
            onChange={e => setPassword(e.target.value)}
            minLength={8}
            className="w-full bg-zinc-900 border border-zinc-700 rounded px-3 py-2 text-sm focus:outline-none focus:border-emerald-500"
            required
          />
        </div>
        <button
          type="submit"
          disabled={loading}
          className="w-full bg-emerald-600 hover:bg-emerald-500 text-white py-2 rounded text-sm disabled:opacity-50"
        >
          {loading ? 'Creating...' : 'Create Account'}
        </button>
      </form>
      <p className="text-sm text-zinc-400 mt-4">
        Already have an account? <Link href="/login" className="text-emerald-400 hover:underline">Login</Link>
      </p>
    </div>
  )
}
