import { useEffect, useState } from 'react'
import { ChevronLeft, ChevronRight, X } from 'lucide-react'
import { getEvents, type EventData } from '../lib/api'
import type { WsEvent } from '../hooks/useWebSocket'

export default function Events({ liveEvents }: { liveEvents: WsEvent[] }) {
  const [events, setEvents] = useState<EventData[]>([])
  const [total, setTotal] = useState(0)
  const [page, setPage] = useState(0)
  const [filter, setFilter] = useState({ status: '', provider: '' })
  const [selected, setSelected] = useState<EventData | null>(null)
  const limit = 25

  useEffect(() => {
    getEvents({ limit, offset: page * limit, status: filter.status || undefined, provider: filter.provider || undefined })
      .then(res => { setEvents(res.events); setTotal(res.total) })
      .catch(() => {})
  }, [page, filter])

  // Merge live events at top for display
  const displayEvents = events.length > 0
    ? events
    : liveEvents.map((e, i) => ({
        id: e.id || e.action_id || String(i),
        timestamp: e.timestamp || new Date().toISOString(),
        agent: e.agent || '',
        action_type: e.action_type || e.type,
        content: e.content || e.explanation || '',
        target: e.target,
        risk_level: e.risk_level,
        matched_rules: e.matched_rules || [],
        provider: undefined,
        status: e.risk_level === 'CRITICAL' || e.risk_level === 'Critical' ? 'blocked' :
                e.risk_level === 'WARNING' || e.risk_level === 'Warning' ? 'warning' : 'passed',
      }))

  const statusColor = (s?: string) => {
    if (s === 'blocked') return 'text-red-400 bg-red-500/20'
    if (s === 'warning') return 'text-amber-400 bg-amber-500/20'
    return 'text-green-400 bg-green-500/20'
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Events Log</h2>

      {/* Filters */}
      <div className="flex gap-3 flex-wrap">
        <select value={filter.status} onChange={e => { setFilter({ ...filter, status: e.target.value }); setPage(0) }}
          className="bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
          <option value="">All Status</option>
          <option value="blocked">Blocked</option>
          <option value="warning">Warning</option>
          <option value="passed">Passed</option>
        </select>
        <select value={filter.provider} onChange={e => { setFilter({ ...filter, provider: e.target.value }); setPage(0) }}
          className="bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500">
          <option value="">All Providers</option>
          <option value="anthropic">Anthropic</option>
          <option value="openai">OpenAI</option>
          <option value="gemini">Gemini</option>
        </select>
      </div>

      {/* Table */}
      <div className="bg-gray-800 rounded-xl border border-gray-700 overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-gray-700/50">
            <tr>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Time</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium hidden md:table-cell">Agent</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Action</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Content</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium">Status</th>
              <th className="text-left px-4 py-3 text-gray-400 font-medium hidden lg:table-cell">Rules</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-700/50">
            {displayEvents.length === 0 && (
              <tr><td colSpan={6} className="text-center py-12 text-gray-500 text-sm">No events recorded yet.</td></tr>
            )}
            {displayEvents.map(ev => (
              <tr key={ev.id} onClick={() => setSelected(ev)} className="hover:bg-gray-700/20 cursor-pointer transition-colors">
                <td className="px-4 py-3 text-xs text-gray-400 whitespace-nowrap">
                  {new Date(ev.timestamp).toLocaleString()}
                </td>
                <td className="px-4 py-3 text-xs text-gray-400 hidden md:table-cell">{ev.agent}</td>
                <td className="px-4 py-3 text-gray-300">{ev.action_type}</td>
                <td className="px-4 py-3 max-w-xs truncate text-gray-300">{ev.content}</td>
                <td className="px-4 py-3">
                  <span className={`px-2 py-0.5 rounded text-xs ${statusColor(ev.status)}`}>
                    {ev.status || 'passed'}
                  </span>
                </td>
                <td className="px-4 py-3 hidden lg:table-cell">
                  {ev.matched_rules.length > 0
                    ? <span className="text-xs text-amber-400">{ev.matched_rules.join(', ')}</span>
                    : <span className="text-xs text-gray-600">—</span>}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {total > limit && (
        <div className="flex items-center justify-between text-sm text-gray-400">
          <span>Showing {page * limit + 1}–{Math.min((page + 1) * limit, total)} of {total}</span>
          <div className="flex gap-2">
            <button onClick={() => setPage(p => Math.max(0, p - 1))} disabled={page === 0}
              className="p-1.5 bg-gray-800 rounded hover:bg-gray-700 disabled:opacity-30 transition-colors">
              <ChevronLeft className="w-4 h-4" />
            </button>
            <button onClick={() => setPage(p => p + 1)} disabled={(page + 1) * limit >= total}
              className="p-1.5 bg-gray-800 rounded hover:bg-gray-700 disabled:opacity-30 transition-colors">
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Detail Modal */}
      {selected && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4" onClick={() => setSelected(null)}>
          <div className="bg-gray-800 rounded-xl border border-gray-700 w-full max-w-2xl max-h-[80vh] overflow-auto" onClick={e => e.stopPropagation()}>
            <div className="flex items-center justify-between px-5 py-4 border-b border-gray-700 sticky top-0 bg-gray-800">
              <h3 className="font-semibold">Event Detail</h3>
              <button onClick={() => setSelected(null)} className="p-1 hover:bg-gray-700 rounded"><X className="w-4 h-4" /></button>
            </div>
            <div className="p-5 space-y-3 text-sm">
              <Row label="ID" value={selected.id} />
              <Row label="Time" value={new Date(selected.timestamp).toLocaleString()} />
              <Row label="Agent" value={selected.agent} />
              <Row label="Action" value={selected.action_type} />
              <Row label="Risk Level" value={selected.risk_level || '—'} />
              <Row label="Status" value={selected.status || 'passed'} />
              <Row label="Matched Rules" value={selected.matched_rules.join(', ') || '—'} />
              <Row label="Target" value={selected.target || '—'} />
              <div>
                <p className="text-xs text-gray-400 mb-1">Content</p>
                <pre className="bg-gray-900 rounded-lg p-3 text-xs text-gray-300 whitespace-pre-wrap break-all max-h-60 overflow-auto">
                  {selected.content}
                </pre>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex gap-4">
      <span className="text-gray-400 w-28 shrink-0">{label}</span>
      <span className="text-gray-200 break-all">{value}</span>
    </div>
  )
}
