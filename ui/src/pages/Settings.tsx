import { useEffect, useState } from 'react'
import { Save, Monitor, ShieldAlert } from 'lucide-react'
import {
  getProxyStatus, updateProxyConfig,
  getProviders, getAlertConfig, updateAlertConfig,
  type ProxyStatus, type ProviderData, type AlertConfig,
} from '../lib/api'

export default function SettingsPage() {
  const [proxy, setProxy] = useState<ProxyStatus | null>(null)
  const [providers, setProviders] = useState<ProviderData[]>([])
  const [alerts, setAlerts] = useState<AlertConfig | null>(null)
  const [saving, setSaving] = useState(false)
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    getProxyStatus().then(setProxy).catch(() => {})
    getProviders().then(setProviders).catch(() => {})
    getAlertConfig().then(setAlerts).catch(() => {})
  }, [])

  const toggleMode = async () => {
    if (!proxy) return
    const newMode = proxy.mode === 'enforce' ? 'monitor' : 'enforce'
    const res = await updateProxyConfig({ mode: newMode })
    setProxy(res)
  }

  const handleSaveAlerts = async () => {
    if (!alerts) return
    setSaving(true)
    try {
      await updateAlertConfig(alerts)
      setSaved(true)
      setTimeout(() => setSaved(false), 2000)
    } catch (e) {
      console.error('Failed to save alerts', e)
    }
    setSaving(false)
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <h2 className="text-2xl font-bold">Settings</h2>

      {/* Proxy Mode */}
      <Section title="Proxy Mode">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            {proxy?.mode === 'enforce'
              ? <ShieldAlert className="w-5 h-5 text-red-400" />
              : <Monitor className="w-5 h-5 text-amber-400" />
            }
            <div>
              <p className="font-medium">{proxy?.mode === 'enforce' ? 'Enforce Mode' : 'Monitor Mode'}</p>
              <p className="text-xs text-gray-400">
                {proxy?.mode === 'enforce'
                  ? 'Dangerous tool_use blocks are actively blocked'
                  : 'All requests pass through, violations are logged only'}
              </p>
            </div>
          </div>
          <button onClick={toggleMode}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              proxy?.mode === 'enforce'
                ? 'bg-red-600 hover:bg-red-700'
                : 'bg-amber-600 hover:bg-amber-700'
            }`}>
            Switch to {proxy?.mode === 'enforce' ? 'Monitor' : 'Enforce'}
          </button>
        </div>
      </Section>

      {/* Providers */}
      <Section title="Providers">
        <div className="space-y-3">
          {providers.map(p => (
            <div key={p.name} className="flex items-center justify-between p-3 bg-gray-900 rounded-lg">
              <div>
                <p className="font-medium text-sm">{p.name}</p>
                <p className="text-xs text-gray-500 font-mono">{p.target_url}</p>
              </div>
              <div className={`px-2 py-1 rounded text-xs ${p.enabled ? 'bg-green-500/20 text-green-400' : 'bg-gray-600/30 text-gray-500'}`}>
                {p.enabled ? 'Active' : 'Inactive'}
              </div>
            </div>
          ))}
        </div>
      </Section>

      {/* Notifications */}
      {alerts && (
        <Section title="Notifications">
          <div className="space-y-5">
            {/* Telegram */}
            <NotifChannel
              emoji="📱" name="Telegram" enabled={alerts.telegram_enabled}
              onToggle={v => setAlerts({ ...alerts, telegram_enabled: v })}
            >
              <Input label="Bot Token" type="password" placeholder="Enter bot token..."
                value={alerts.telegram_bot_token || ''} disabled={!alerts.telegram_enabled}
                onChange={v => setAlerts({ ...alerts, telegram_bot_token: v })} />
              <Input label="Chat ID" placeholder="Enter chat ID..."
                value={alerts.telegram_chat_id || ''} disabled={!alerts.telegram_enabled}
                onChange={v => setAlerts({ ...alerts, telegram_chat_id: v })} />
            </NotifChannel>

            {/* Slack */}
            <NotifChannel
              emoji="💬" name="Slack" enabled={alerts.slack_enabled}
              onToggle={v => setAlerts({ ...alerts, slack_enabled: v })}
            >
              <Input label="Webhook URL" placeholder="https://hooks.slack.com/..."
                value={alerts.slack_webhook || ''} disabled={!alerts.slack_enabled}
                onChange={v => setAlerts({ ...alerts, slack_webhook: v })} />
            </NotifChannel>

            {/* Discord */}
            <NotifChannel
              emoji="🎮" name="Discord" enabled={alerts.discord_enabled}
              onToggle={v => setAlerts({ ...alerts, discord_enabled: v })}
            >
              <Input label="Webhook URL" placeholder="https://discord.com/api/webhooks/..."
                value={alerts.discord_webhook || ''} disabled={!alerts.discord_enabled}
                onChange={v => setAlerts({ ...alerts, discord_webhook: v })} />
            </NotifChannel>

            {/* Levels */}
            <div className="pt-2">
              <p className="text-xs text-gray-400 mb-3">Notification Levels</p>
              <div className="space-y-2">
                <Toggle label="🚨 Critical" desc="Blocked actions" checked={alerts.notify_on_critical}
                  onChange={v => setAlerts({ ...alerts, notify_on_critical: v })} />
                <Toggle label="⚠️ Warning" desc="Suspicious actions" checked={alerts.notify_on_warning}
                  onChange={v => setAlerts({ ...alerts, notify_on_warning: v })} />
                <Toggle label="ℹ️ Info" desc="All monitored actions" checked={alerts.notify_on_info}
                  onChange={v => setAlerts({ ...alerts, notify_on_info: v })} />
              </div>
            </div>

            <button onClick={handleSaveAlerts} disabled={saving}
              className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 px-5 py-2.5 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
              <Save className="w-4 h-4" />
              {saved ? 'Saved!' : saving ? 'Saving...' : 'Save Settings'}
            </button>
          </div>
        </Section>
      )}
    </div>
  )
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="bg-gray-800 rounded-xl border border-gray-700 p-5">
      <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wide mb-4">{title}</h3>
      {children}
    </div>
  )
}

function NotifChannel({ emoji, name, enabled, onToggle, children }: {
  emoji: string; name: string; enabled: boolean; onToggle: (v: boolean) => void; children: React.ReactNode
}) {
  return (
    <div className="p-3 bg-gray-900 rounded-lg space-y-3">
      <div className="flex items-center justify-between">
        <span className="font-medium text-sm">{emoji} {name}</span>
        <button onClick={() => onToggle(!enabled)}
          className={`w-10 h-5 rounded-full relative transition-colors ${enabled ? 'bg-green-500' : 'bg-gray-600'}`}>
          <div className={`w-4 h-4 bg-white rounded-full absolute top-0.5 transition-all ${enabled ? 'left-5' : 'left-0.5'}`} />
        </button>
      </div>
      <div className={`space-y-3 ${!enabled ? 'opacity-40 pointer-events-none' : ''}`}>
        {children}
      </div>
    </div>
  )
}

function Input({ label, placeholder, value, onChange, disabled, type = 'text' }: {
  label: string; placeholder: string; value: string; onChange: (v: string) => void; disabled?: boolean; type?: string
}) {
  return (
    <div>
      <label className="block text-xs text-gray-400 mb-1">{label}</label>
      <input type={type} value={value} onChange={e => onChange(e.target.value)} placeholder={placeholder} disabled={disabled}
        className="w-full bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500 disabled:opacity-50" />
    </div>
  )
}

function Toggle({ label, desc, checked, onChange }: {
  label: string; desc: string; checked: boolean; onChange: (v: boolean) => void
}) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={e => onChange(e.target.checked)} className="w-4 h-4 rounded" />
      <span className="text-sm">{label}</span>
      <span className="text-xs text-gray-500">{desc}</span>
    </label>
  )
}
