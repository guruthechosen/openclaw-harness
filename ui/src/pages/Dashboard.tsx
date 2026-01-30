import { useEffect, useState } from 'react'
import { Activity, ShieldX, AlertTriangle, CheckCircle, Cpu, Clock } from 'lucide-react'
import { getStats, getProxyStatus, type Stats, type ProxyStatus } from '../lib/api'
import type { WsEvent } from '../hooks/useWebSocket'

function StatCard({ title, value, icon: Icon, color }: {
  title: string; value: number | string; icon: React.ElementType; color: string
}) {
  const colors: Record<string, string> = {
    blue: 'border-blue-500 bg-blue-500/10 text-blue-400',
    red: 'border-red-500 bg-red-500/10 text-red-400',
    amber: 'border-amber-500 bg-amber-500/10 text-amber-400',
    green: 'border-green-500 bg-green-500/10 text-green-400',
  }
  return (
    <div className={`border rounded-xl p-4 ${colors[color]}`}>
      <div className="flex items-center justify-between mb-2">
        <span className="text-xs text-gray-400 uppercase tracking-wide">{title}</span>
        <Icon className="w-4 h-4 opacity-60" />
      </div>
      <p className="text-2xl font-bold text-gray-100">{value}</p>
    </div>
  )
}

function formatUptime(seconds: number) {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

export default function Dashboard({ events }: { events: WsEvent[] }) {
  const [stats, setStats] = useState<Stats | null>(null)
  const [proxy, setProxy] = useState<ProxyStatus | null>(null)

  useEffect(() => {
    getStats().then(setStats).catch(() => {})
    getProxyStatus().then(setProxy).catch(() => {})
    const iv = setInterval(() => {
      getStats().then(setStats).catch(() => {})
      getProxyStatus().then(setProxy).catch(() => {})
    }, 5000)
    return () => clearInterval(iv)
  }, [])

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Dashboard</h2>

      {/* Stats */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="Total Requests" value={stats?.total_events ?? 0} icon={Activity} color="blue" />
        <StatCard title="Blocked" value={stats?.blocked_count ?? 0} icon={ShieldX} color="red" />
        <StatCard title="Warnings" value={stats?.warning_count ?? 0} icon={AlertTriangle} color="amber" />
        <StatCard title="Passed" value={stats?.passed_count ?? 0} icon={CheckCircle} color="green" />
      </div>

      {/* Proxy Status */}
      {proxy && (
        <div className="bg-gray-800 rounded-xl p-5 border border-gray-700">
          <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wide mb-3">Proxy Status</h3>
          <div className="flex flex-wrap gap-6 text-sm">
            <div className="flex items-center gap-2">
              <div className={`w-2.5 h-2.5 rounded-full ${proxy.running ? 'bg-green-500' : 'bg-red-500'}`} />
              <span>{proxy.running ? 'Running' : 'Stopped'}</span>
            </div>
            <div className="flex items-center gap-2">
              <Cpu className="w-4 h-4 text-gray-500" />
              <span>Mode: <span className={`font-medium ${proxy.mode === 'enforce' ? 'text-red-400' : 'text-amber-400'}`}>
                {proxy.mode === 'enforce' ? 'Enforce' : 'Monitor'}
              </span></span>
            </div>
            <div className="flex items-center gap-2">
              <Clock className="w-4 h-4 text-gray-500" />
              <span>Uptime: {formatUptime(proxy.uptime_seconds)}</span>
            </div>
          </div>
        </div>
      )}

      {/* Recent Events */}
      <div className="bg-gray-800 rounded-xl border border-gray-700">
        <div className="px-5 py-4 border-b border-gray-700">
          <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wide">Recent Events</h3>
        </div>
        <div className="divide-y divide-gray-700/50">
          {events.length === 0 && (
            <p className="text-gray-500 text-center py-12 text-sm">No events yet. Waiting for agent activity...</p>
          )}
          {events.slice(0, 20).map((ev, i) => {
            const riskColor = ev.risk_level === 'CRITICAL' || ev.risk_level === 'Critical'
              ? 'bg-red-500'
              : ev.risk_level === 'WARNING' || ev.risk_level === 'Warning'
              ? 'bg-amber-500'
              : 'bg-green-500'
            return (
              <div key={ev.id || ev.action_id || i} className="flex items-center gap-3 px-5 py-3 hover:bg-gray-700/30 transition-colors">
                <div className={`w-2 h-2 rounded-full shrink-0 ${riskColor}`} />
                <div className="flex-1 min-w-0">
                  <p className="text-sm truncate text-gray-200">{ev.content || ev.explanation || 'Event'}</p>
                  <p className="text-xs text-gray-500">
                    {ev.agent && <span>{ev.agent} · </span>}
                    {ev.action_type || ev.type}
                    {ev.matched_rules && ev.matched_rules.length > 0 && (
                      <span className="text-amber-400"> · {ev.matched_rules.join(', ')}</span>
                    )}
                  </p>
                </div>
                <span className="text-xs text-gray-500 shrink-0">
                  {ev.timestamp ? new Date(ev.timestamp).toLocaleTimeString() : ''}
                </span>
              </div>
            )
          })}
        </div>
      </div>
    </div>
  )
}
