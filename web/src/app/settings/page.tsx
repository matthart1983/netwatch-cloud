'use client'

import { useState, useEffect } from 'react'
import { useRouter } from 'next/navigation'
import { useAuth } from '@/lib/auth'
import { getApiKeys, createApiKey, deleteApiKey, ApiKeyInfo, getAccount, updateAccount, AccountInfo } from '@/lib/api'

export default function SettingsPage() {
  const { token, isLoading: authLoading } = useAuth()
  const router = useRouter()
  const [keys, setKeys] = useState<ApiKeyInfo[]>([])
  const [newKey, setNewKey] = useState<string | null>(null)
  const [account, setAccount] = useState<AccountInfo | null>(null)
  const [notifyEmail, setNotifyEmail] = useState(true)
  const [slackWebhook, setSlackWebhook] = useState('')
  const [slackWebhookDirty, setSlackWebhookDirty] = useState(false)
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (authLoading) return
    if (!token) { router.push('/login'); return }
    loadData()
  }, [authLoading, token, router])

  async function loadData() {
    try {
      const [keysData, accountData] = await Promise.all([getApiKeys(), getAccount()])
      setKeys(keysData)
      setAccount(accountData)
      setNotifyEmail(accountData.notify_email)
      setSlackWebhook('')
      setSlackWebhookDirty(false)
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
      loadData()
    } catch {
      // handled
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Revoke this API key? Agents using it will stop sending data.')) return
    try {
      await deleteApiKey(id)
      loadData()
    } catch {
      // handled
    }
  }

  async function handleSaveNotifications() {
    setSaving(true)
    setSaved(false)
    try {
      const update: { notify_email?: boolean; slack_webhook?: string } = {
        notify_email: notifyEmail,
      }

      if (slackWebhookDirty) {
        update.slack_webhook = slackWebhook
      }

      await updateAccount(update)
      await loadData()
      setSaved(true)
      setTimeout(() => setSaved(false), 3000)
    } catch {
      // handled
    } finally {
      setSaving(false)
    }
  }

  function getTrialDaysLeft(): number {
    if (!account?.trial_ends_at) return 0
    const diff = new Date(account.trial_ends_at).getTime() - Date.now()
    return Math.max(0, Math.ceil(diff / (1000 * 60 * 60 * 24)))
  }

  const slackWebhookWillBeRemoved = Boolean(account?.has_slack_webhook) && slackWebhookDirty && slackWebhook.trim() === ''

  if (authLoading || loading) return <div className="text-zinc-400 mt-10">Loading...</div>

  return (
    <div className="max-w-2xl">
      <h1 className="text-2xl font-bold mb-6">Settings</h1>

      {/* Account & Subscription */}
      {account && (
        <section className="mb-8">
          <h2 className="text-lg font-semibold mb-4">Account</h2>
          <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <div className="text-sm text-zinc-400">Email</div>
                <div className="text-sm font-medium">{account.email}</div>
              </div>
              <div>
                <div className="text-sm text-zinc-400">Member since</div>
                <div className="text-sm font-medium">{new Date(account.created_at).toLocaleDateString()}</div>
              </div>
            </div>

            <div className="border-t border-zinc-800 pt-4">
              <div className="flex items-center gap-3 mb-2">
                <span className={`px-2 py-0.5 rounded text-xs font-medium ${
                  account.plan === 'early_access' ? 'bg-emerald-900 text-emerald-300' :
                  account.plan === 'trial' ? 'bg-yellow-900 text-yellow-300' :
                  account.plan === 'past_due' ? 'bg-orange-900 text-orange-300' :
                  'bg-red-900 text-red-300'
                }`}>
                  {account.plan === 'early_access' ? 'Early Access' :
                   account.plan === 'trial' ? 'Trial' :
                   account.plan === 'past_due' ? 'Past Due' : 'Expired'}
                </span>
                <span className="text-sm text-zinc-400">
                  {account.plan === 'trial' && `${getTrialDaysLeft()} days remaining`}
                  {account.plan === 'early_access' && '$49/mo'}
                  {account.plan === 'expired' && 'Add a payment method to continue'}
                  {account.plan === 'past_due' && 'Update your payment method'}
                </span>
              </div>
              <p className="text-xs text-zinc-500 mb-3">
                {account.plan === 'early_access'
                  ? '10 hosts · 72h data retention · Email + Slack alerts'
                  : '3 hosts · 24h data retention · Email alerts only'}
              </p>
              <p className="text-xs text-zinc-500 mb-3">
                Billing changes happen in Stripe&apos;s hosted portal.
              </p>
              {account.portal_url ? (
                <a
                  href={account.portal_url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded text-sm inline-block"
                >
                  Open Stripe Billing →
                </a>
              ) : (
                <button
                  disabled
                  className="bg-zinc-700 text-zinc-400 px-4 py-2 rounded text-sm cursor-not-allowed"
                >
                  Billing Portal Unavailable
                </button>
              )}
            </div>
          </div>
        </section>
      )}

      {/* Notifications */}
      <section className="mb-8">
        <h2 className="text-lg font-semibold mb-4">Notifications</h2>
        <div className="bg-zinc-900 border border-zinc-800 rounded-lg p-4 space-y-4">
          <label className="flex items-center justify-between cursor-pointer">
            <div>
              <div className="text-sm font-medium">Email Alerts</div>
              <div className="text-xs text-zinc-500">Receive alert notifications via email</div>
            </div>
            <button
              onClick={() => setNotifyEmail(!notifyEmail)}
              className={`relative w-10 h-5 rounded-full transition-colors ${notifyEmail ? 'bg-emerald-600' : 'bg-zinc-700'}`}
            >
              <span className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full transition-transform ${notifyEmail ? 'translate-x-5' : ''}`} />
            </button>
          </label>

          <div>
            <label className="block text-sm font-medium mb-1">Slack Webhook URL</label>
            <p className="text-xs text-zinc-500 mb-2">
              {account?.has_slack_webhook && !slackWebhookDirty
                ? 'A webhook is already saved. Leave this blank to keep it, or enter a new one to replace it.'
                : 'Receive alert notifications in a Slack channel.'}
            </p>
            {account?.has_slack_webhook && !slackWebhookDirty && (
              <p className="text-xs text-emerald-400 mb-2">Slack webhook configured.</p>
            )}
            {slackWebhookWillBeRemoved && (
              <p className="text-xs text-orange-400 mb-2">Slack webhook will be removed when you save.</p>
            )}
            <input
              type="url"
              value={slackWebhook}
              onChange={e => {
                setSlackWebhook(e.target.value)
                setSlackWebhookDirty(true)
              }}
              placeholder={account?.has_slack_webhook ? 'Enter a new Slack webhook to replace the saved one' : 'https://hooks.slack.com/services/...'}
              className="w-full bg-zinc-950 border border-zinc-700 rounded px-3 py-2 text-sm focus:outline-none focus:border-emerald-500 placeholder-zinc-600"
            />
            {account?.has_slack_webhook && (
              <button
                type="button"
                onClick={() => {
                  setSlackWebhook('')
                  setSlackWebhookDirty(true)
                }}
                className="mt-2 text-xs text-zinc-400 hover:text-zinc-100"
              >
                Remove saved Slack webhook
              </button>
            )}
          </div>

          <button
            onClick={handleSaveNotifications}
            disabled={saving}
            className="bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2 rounded text-sm disabled:opacity-50"
          >
            {saving ? 'Saving...' : saved ? '✓ Saved' : 'Save Notification Settings'}
          </button>
        </div>
      </section>

      {/* API Keys */}
      <section className="mb-8">
        <h2 className="text-lg font-semibold mb-4">API Keys</h2>

        {newKey && (
          <div className="bg-zinc-900 border border-emerald-700 rounded-lg p-4 mb-4">
            <p className="text-sm text-emerald-400 mb-2">New API key created (shown once):</p>
            <div className="font-mono text-sm break-all mb-2">{newKey}</div>

            <p className="text-sm text-zinc-400 mt-3 mb-2">Install command (ready to paste on your Linux server):</p>
            <div className="bg-zinc-950 border border-zinc-800 rounded p-3 font-mono text-xs break-all mb-3 select-all">
              curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sudo sh -s -- --api-key {newKey} --endpoint https://netwatch-api-production.up.railway.app/api/v1/ingest
            </div>

            <div className="flex gap-2">
              <button
                onClick={() => navigator.clipboard.writeText(`curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sudo sh -s -- --api-key ${newKey} --endpoint https://netwatch-api-production.up.railway.app/api/v1/ingest`)}
                className="bg-emerald-600 hover:bg-emerald-500 text-white px-3 py-1 rounded text-xs"
              >
                Copy Install Command
              </button>
              <button
                onClick={() => navigator.clipboard.writeText(newKey)}
                className="bg-zinc-700 hover:bg-zinc-600 text-white px-3 py-1 rounded text-xs"
              >
                Copy Key Only
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

      {/* Install Agent */}
      <section className="mb-8">
        <h2 className="text-lg font-semibold mb-4">Install Agent</h2>
        <p className="text-zinc-400 text-sm mb-2">
          Create a new API key above — the full install command will appear automatically with your key pre-filled.
        </p>
        <div className="bg-zinc-900 border border-zinc-800 rounded p-3 font-mono text-xs break-all text-zinc-500">
          curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sudo sh -s -- --api-key <span className="text-yellow-400">YOUR_API_KEY</span> --endpoint https://netwatch-api-production.up.railway.app/api/v1/ingest
        </div>
      </section>

      {/* Agent Commands */}
      <section>
        <h2 className="text-lg font-semibold mb-4">Agent Commands</h2>
        <div className="bg-zinc-900 border border-zinc-800 rounded p-3 font-mono text-xs space-y-1">
          <div><span className="text-zinc-500"># Check status</span></div>
          <div>netwatch-agent status</div>
          <div className="pt-1"><span className="text-zinc-500"># View config</span></div>
          <div>netwatch-agent config</div>
          <div className="pt-1"><span className="text-zinc-500"># Update to latest version</span></div>
          <div>sudo netwatch-agent update</div>
          <div className="pt-1"><span className="text-zinc-500"># View logs</span></div>
          <div>journalctl -u netwatch-agent -f</div>
          <div className="pt-1"><span className="text-zinc-500"># Remove agent</span></div>
          <div>curl -sSL https://netwatch-api-production.up.railway.app/install.sh | sudo sh -s -- --remove</div>
        </div>
      </section>
    </div>
  )
}
