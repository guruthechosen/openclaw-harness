import { useEffect, useState } from 'react'
import { Plus, Pencil, Trash2, X, FlaskConical, Lock } from 'lucide-react'
import { getRules, createRule, updateRule, deleteRule, testRule, type RuleData } from '../lib/api'

function RiskBadge({ level }: { level: string }) {
  const cls = level === 'Critical'
    ? 'bg-red-500/20 text-red-400 border-red-500/30'
    : level === 'Warning'
    ? 'bg-amber-500/20 text-amber-400 border-amber-500/30'
    : 'bg-blue-500/20 text-blue-400 border-blue-500/30'
  return <span className={`px-2 py-0.5 rounded text-xs border ${cls}`}>{level}</span>
}

function ActionBadge({ action }: { action: string }) {
  const map: Record<string, string> = {
    CriticalAlert: 'bg-red-500/20 text-red-300',
    PauseAndAsk: 'bg-amber-500/20 text-amber-300',
    Alert: 'bg-yellow-500/20 text-yellow-300',
    LogOnly: 'bg-gray-500/20 text-gray-300',
  }
  const labels: Record<string, string> = {
    CriticalAlert: '🛑 Block + Alert',
    PauseAndAsk: '⚠️ Block + Review',
    Alert: '🔔 Alert Only',
    LogOnly: '📝 Log Only',
  }
  return <span className={`px-2 py-0.5 rounded text-xs ${map[action] || map.LogOnly}`}>{labels[action] || action}</span>
}

interface ModalState {
  open: boolean
  editing: RuleData | null
}

export default function Rules() {
  const [rules, setRules] = useState<RuleData[]>([])
  const [modal, setModal] = useState<ModalState>({ open: false, editing: null })
  const [form, setForm] = useState({ name: '', description: '', pattern: '', risk_level: 'Warning', action: 'Alert' })
  const [testInput, setTestInput] = useState('')
  const [testResult, setTestResult] = useState<{ matches: boolean; matched_text: string | null } | null>(null)
  const [saving, setSaving] = useState(false)

  const load = () => getRules().then(setRules).catch(() => {})
  useEffect(() => { load() }, [])

  const openCreate = () => {
    setForm({ name: '', description: '', pattern: '', risk_level: 'Warning', action: 'Alert' })
    setTestInput('')
    setTestResult(null)
    setModal({ open: true, editing: null })
  }

  const openEdit = (r: RuleData) => {
    setForm({ name: r.name, description: r.description, pattern: r.pattern, risk_level: r.risk_level, action: r.action })
    setTestInput('')
    setTestResult(null)
    setModal({ open: true, editing: r })
  }

  const handleSave = async () => {
    setSaving(true)
    try {
      if (modal.editing) {
        await updateRule(modal.editing.name, {
          description: form.description,
          pattern: form.pattern,
          risk_level: form.risk_level,
          action: form.action,
        })
      } else {
        await createRule(form)
      }
      setModal({ open: false, editing: null })
      load()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Failed to save rule')
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (name: string) => {
    if (!confirm(`Delete rule "${name}"?`)) return
    try {
      await deleteRule(name)
      load()
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : 'Failed to delete rule')
    }
  }

  const handleToggle = async (r: RuleData) => {
    try {
      await updateRule(r.name, { enabled: !r.enabled })
      load()
    } catch (e) {
      console.error('Failed to toggle rule', e)
    }
  }

  const handleTest = async () => {
    if (!form.pattern || !testInput) return
    try {
      const res = await testRule(form.pattern, testInput)
      setTestResult(res)
    } catch {
      setTestResult({ matches: false, matched_text: null })
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Rules</h2>
        <button onClick={openCreate} className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded-lg text-sm transition-colors">
          <Plus className="w-4 h-4" /> Add Rule
        </button>
      </div>

      {/* Table */}
      <div className="bg-gray-800 rounded-xl border border-gray-700 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-gray-700/50">
            <tr>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Name</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium hidden lg:table-cell">Pattern</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Risk</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Action</th>
              <th className="text-center px-4 py-3 text-gray-400 font-medium">Enabled</th>
              <th className="text-right px-4 py-3 text-gray-400 font-medium">Actions</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-700/50">
            {rules.map(r => (
              <tr key={r.name} className="hover:bg-gray-700/20 transition-colors">
                <td className="px-4 py-3">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-gray-200">{r.name}</span>
                    {r.is_preset && <Lock className="w-3 h-3 text-gray-500" />}
                  </div>
                  <p className="text-xs text-gray-500 mt-0.5">{r.description}</p>
                </td>
                <td className="px-4 py-3 hidden lg:table-cell">
                  <code className="text-xs text-gray-400 bg-gray-900 px-2 py-1 rounded break-all">
                    {r.pattern.length > 40 ? r.pattern.slice(0, 40) + '…' : r.pattern}
                  </code>
                </td>
                <td className="px-4 py-3"><RiskBadge level={r.risk_level} /></td>
                <td className="px-4 py-3"><ActionBadge action={r.action} /></td>
                <td className="px-4 py-3 text-center">
                  <button
                    onClick={() => handleToggle(r)}
                    className={`w-10 h-5 rounded-full relative transition-colors ${r.enabled ? 'bg-green-500' : 'bg-gray-600'}`}
                  >
                    <div className={`w-4 h-4 bg-white rounded-full absolute top-0.5 transition-all ${r.enabled ? 'left-5' : 'left-0.5'}`} />
                  </button>
                </td>
                <td className="px-4 py-3 text-right">
                  <div className="flex items-center justify-end gap-1">
                    <button onClick={() => openEdit(r)} className="p-1.5 hover:bg-gray-600 rounded transition-colors" title="Edit">
                      <Pencil className="w-3.5 h-3.5 text-gray-400" />
                    </button>
                    {!r.is_preset && (
                      <button onClick={() => handleDelete(r.name)} className="p-1.5 hover:bg-red-900/50 rounded transition-colors" title="Delete">
                        <Trash2 className="w-3.5 h-3.5 text-red-400" />
                      </button>
                    )}
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Modal */}
      {modal.open && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
          <div className="bg-gray-800 rounded-xl border border-gray-700 w-full max-w-lg">
            <div className="flex items-center justify-between px-5 py-4 border-b border-gray-700">
              <h3 className="font-semibold">{modal.editing ? 'Edit Rule' : 'New Rule'}</h3>
              <button onClick={() => setModal({ open: false, editing: null })} className="p-1 hover:bg-gray-700 rounded">
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5 space-y-4">
              {!modal.editing && (
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Name</label>
                  <input value={form.name} onChange={e => setForm({ ...form, name: e.target.value })}
                    className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
                </div>
              )}
              <div>
                <label className="block text-xs text-gray-400 mb-1">Description</label>
                <input value={form.description} onChange={e => setForm({ ...form, description: e.target.value })}
                  className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500" />
              </div>
              <div>
                <label className="block text-xs text-gray-400 mb-1">Pattern (regex)</label>
                <input value={form.pattern} onChange={e => setForm({ ...form, pattern: e.target.value })}
                  className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm font-mono focus:outline-none focus:border-blue-500" />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Risk Level</label>
                  <select value={form.risk_level} onChange={e => setForm({ ...form, risk_level: e.target.value })}
                    className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
                    <option value="Critical">Critical</option>
                    <option value="Warning">Warning</option>
                    <option value="Info">Info</option>
                  </select>
                </div>
                <div>
                  <label className="block text-xs text-gray-400 mb-1">Action</label>
                  <select value={form.action} onChange={e => setForm({ ...form, action: e.target.value })}
                    className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
                    <option value="CriticalAlert">🛑 Block + Alert</option>
                    <option value="PauseAndAsk">⚠️ Block + Review</option>
                    <option value="Alert">Alert</option>
                    <option value="LogOnly">Log Only</option>
                  </select>
                </div>
              </div>

              {/* Test */}
              <div className="bg-gray-900 rounded-lg p-3 space-y-2">
                <div className="flex items-center gap-2 text-xs text-gray-400">
                  <FlaskConical className="w-3.5 h-3.5" /> Test Pattern
                </div>
                <div className="flex gap-2">
                  <input value={testInput} onChange={e => setTestInput(e.target.value)} placeholder="Test input..."
                    className="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-1.5 text-sm font-mono focus:outline-none focus:border-blue-500" />
                  <button onClick={handleTest} className="px-3 py-1.5 bg-gray-700 hover:bg-gray-600 rounded text-sm transition-colors">Test</button>
                </div>
                {testResult && (
                  <p className={`text-xs ${testResult.matches ? 'text-red-400' : 'text-green-400'}`}>
                    {testResult.matches
                      ? `✓ Match: "${testResult.matched_text}"`
                      : '✗ No match'}
                  </p>
                )}
              </div>
            </div>
            <div className="flex justify-end gap-2 px-5 py-4 border-t border-gray-700">
              <button onClick={() => setModal({ open: false, editing: null })}
                className="px-4 py-2 text-sm text-gray-400 hover:text-gray-200 transition-colors">Cancel</button>
              <button onClick={handleSave} disabled={saving}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition-colors disabled:opacity-50">
                {saving ? 'Saving...' : 'Save'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
