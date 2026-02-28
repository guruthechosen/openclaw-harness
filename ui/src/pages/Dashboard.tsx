import { useEffect, useState, useMemo } from 'react'
import { Link } from 'react-router-dom'
import {
  Activity, ShieldX, AlertTriangle, CheckCircle, Server, Eye,
  Download, ArrowUpRight, ArrowDownRight, Shield, TrendingUp
} from 'lucide-react'
import {
  AreaChart, Area, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  PieChart, Pie, Cell, Legend
} from 'recharts'
import { getStats, getProxyStatus, updateProxyConfig, type Stats, type ProxyStatus } from '../lib/api'
import type { WsEvent } from '../hooks/useWebSocket'

const iconBg: Record<string, string> = {
  blue: 'bg-blue-500/15',
  red: 'bg-red-500/15',
  amber: 'bg-amber-500/15',
  green: 'bg-green-500/15',
}
const iconText: Record<string, string> = {
  blue: 'text-blue-400',
  red: 'text-red-400',
  amber: 'text-amber-400',
  green: 'text-green-400',
}

function StatCard({ title, value, icon: Icon, color, change }: {
  title: string; value: number | string; icon: React.ElementType; color: string; change?: number
}) {
  const up = change != null && change >= 0
  return (
    <div className="bg-gray-800/50 border border-gray-700/50 rounded-xl p-5">
      <div className="flex items-center justify-between mb-3">
        <div className={`w-10 h-10 rounded-full flex items-center justify-center ${iconBg[color]}`}>
          <Icon className={`w-5 h-5 ${iconText[color]}`} />
        </div>
        {change != null && (
          <span className={`flex items-center gap-0.5 text-xs font-medium px-2 py-0.5 rounded-full ${
            up ? 'bg-green-500/10 text-green-400' : 'bg-red-500/10 text-red-400'
          }`}>
            {up ? <ArrowUpRight className="w-3 h-3" /> : <ArrowDownRight className="w-3 h-3" />}
            {up ? '+' : ''}{change}%
          </span>
        )}
      </div>
      <p className="text-sm text-gray-400 mb-1">{title}</p>
      <p className="text-3xl font-bold text-white">{typeof value === 'number' ? value.toLocaleString() : value}</p>
    </div>
  )
}

function formatUptime(seconds: number) {
  const d = Math.floor(seconds / 86400)
  const h = Math.floor((seconds % 86400) / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  if (d > 0) return `${d}d ${h}h ${m}m`
  if (h > 0) return `${h}h ${m}m`
  return `${m}m`
}

const riskConfig: Record<string, { dot: string; text: string; label: string }> = {
  CRITICAL: { dot: 'bg-red-500', text: 'text-red-400', label: 'Critical' },
  Critical: { dot: 'bg-red-500', text: 'text-red-400', label: 'Critical' },
  WARNING: { dot: 'bg-amber-500', text: 'text-amber-400', label: 'Warning' },
  Warning: { dot: 'bg-amber-500', text: 'text-amber-400', label: 'Warning' },
  INFO: { dot: 'bg-blue-500', text: 'text-blue-400', label: 'Info' },
  Info: { dot: 'bg-blue-500', text: 'text-blue-400', label: 'Info' },
  LOW: { dot: 'bg-green-500', text: 'text-green-400', label: 'Low' },
  Low: { dot: 'bg-green-500', text: 'text-green-400', label: 'Low' },
}

const tagColors: Record<string, string> = {
  MALICIOUS_INTENT: 'bg-red-500/15 text-red-400 border-red-500/30',
  DATA_PRIVACY: 'bg-amber-500/15 text-amber-400 border-amber-500/30',
  POLICY_KEYWORD: 'bg-blue-500/15 text-blue-400 border-blue-500/30',
  COMPLIANCE: 'bg-green-500/15 text-green-400 border-green-500/30',
}

function RuleTag({ tag }: { tag: string }) {
  const cls = tagColors[tag] || 'bg-gray-700/50 text-gray-400 border-gray-600/30'
  return (
    <span className={`text-[11px] font-medium px-2 py-0.5 rounded-md border ${cls}`}>
      {tag}
    </span>
  )
}

// Demo data for when no real events exist
function generateDemoTrend() {
  const hours = []
  const now = new Date()
  for (let i = 23; i >= 0; i--) {
    const h = new Date(now.getTime() - i * 3600000)
    const label = h.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    const base = Math.floor(Math.random() * 40) + 10
    hours.push({
      time: label,
      total: base + Math.floor(Math.random() * 30),
      blocked: Math.floor(Math.random() * 8),
      warnings: Math.floor(Math.random() * 5),
      passed: base + Math.floor(Math.random() * 20),
    })
  }
  return hours
}

const DEMO_TREND = generateDemoTrend()

const PIE_COLORS = ['#ef4444', '#f59e0b', '#3b82f6', '#22c55e']

const CustomTooltip = ({ active, payload, label }: { active?: boolean; payload?: Array<{ color?: string; name?: string; value?: number }>; label?: string }) => {
  if (!active || !payload?.length) return null
  return (
    <div className="bg-gray-800 border border-gray-700 rounded-lg px-3 py-2 shadow-xl">
      <p className="text-xs text-gray-400 mb-1">{label}</p>
      {payload.map((p, i: number) => (
        <p key={i} className="text-xs" style={{ color: p.color }}>
          {p.name}: <span className="font-semibold">{p.value}</span>
        </p>
      ))}
    </div>
  )
}

export default function Dashboard({ events }: { events: WsEvent[] }) {
  const [stats, setStats] = useState<Stats | null>(null)
  const [proxy, setProxy] = useState<ProxyStatus | null>(null)
  const [chartPeriod, setChartPeriod] = useState<'24h' | '7d' | '30d'>('24h')

  useEffect(() => {
    getStats().then(setStats).catch(() => {})
    getProxyStatus().then(setProxy).catch(() => {})
    const iv = setInterval(() => {
      getStats().then(setStats).catch(() => {})
      getProxyStatus().then(setProxy).catch(() => {})
    }, 5000)
    return () => clearInterval(iv)
  }, [])

  // Build trend data from real events, fallback to demo
  const trendData = useMemo(() => {
    if (events.length === 0) return DEMO_TREND
    const buckets: Record<string, { total: number; blocked: number; warnings: number; passed: number }> = {}
    const now = new Date()
    for (let i = 23; i >= 0; i--) {
      const h = new Date(now.getTime() - i * 3600000)
      const label = h.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      buckets[label] = { total: 0, blocked: 0, warnings: 0, passed: 0 }
    }
    for (const ev of events) {
      if (!ev.timestamp) continue
      const h = new Date(ev.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
      if (buckets[h]) {
        buckets[h].total++
        const rl = (ev.risk_level || '').toUpperCase()
        if (rl === 'CRITICAL') buckets[h].blocked++
        else if (rl === 'WARNING') buckets[h].warnings++
        else buckets[h].passed++
      }
    }
    return Object.entries(buckets).map(([time, d]) => ({ time, ...d }))
  }, [events])

  // Pie data from stats
  const pieData = useMemo(() => {
    if (!stats || stats.total_events === 0) {
      return [
        { name: 'Critical', value: 12 },
        { name: 'Warning', value: 23 },
        { name: 'Info', value: 45 },
        { name: 'Passed', value: 120 },
      ]
    }
    return [
      { name: 'Critical', value: stats.critical_count },
      { name: 'Warning', value: stats.warning_count },
      { name: 'Info', value: stats.info_count },
      { name: 'Passed', value: stats.passed_count },
    ].filter(d => d.value > 0)
  }, [stats])

  const isDemo = events.length === 0

  const toggleProxy = () => {
    if (!proxy) return
    updateProxyConfig({ enabled: !proxy.running })
      .then(setProxy)
      .catch(() => {})
  }

  const setMode = (mode: string) => {
    updateProxyConfig({ mode })
      .then(setProxy)
      .catch(() => {})
  }

  const total = stats?.total_events ?? 0

  return (
    <div className="space-y-6 max-w-[1400px]">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Security Overview</h1>
          <p className="text-sm text-gray-400 mt-1">Real-time monitoring for AI agent interactions</p>
        </div>
        <button className="flex items-center gap-2 px-4 py-2 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm text-gray-300 transition-colors">
          <Download className="w-4 h-4" />
          Export Logs
        </button>
      </div>

      {/* Stat Cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard title="Total Requests" value={total} icon={Activity} color="blue" change={9.5} />
        <StatCard title="Blocked" value={stats?.blocked_count ?? 0} icon={ShieldX} color="red" change={total > 0 ? -2.3 : undefined} />
        <StatCard title="Warnings" value={stats?.warning_count ?? 0} icon={AlertTriangle} color="amber" change={total > 0 ? 5.1 : undefined} />
        <StatCard title="Passed" value={stats?.passed_count ?? 0} icon={CheckCircle} color="green" change={total > 0 ? 12.4 : undefined} />
      </div>

      {/* System Status */}
      {proxy && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {/* Proxy Service */}
          <div className="bg-gray-800/50 border border-gray-700/50 rounded-xl p-5">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className="w-9 h-9 rounded-full bg-blue-500/15 flex items-center justify-center">
                  <Server className="w-4 h-4 text-blue-400" />
                </div>
                <h3 className="text-sm font-semibold text-white">Proxy Service</h3>
              </div>
              <button
                onClick={toggleProxy}
                className={`relative w-11 h-6 rounded-full transition-colors ${
                  proxy.running ? 'bg-green-500' : 'bg-gray-600'
                }`}
              >
                <div className={`absolute top-0.5 w-5 h-5 bg-white rounded-full shadow transition-transform ${
                  proxy.running ? 'left-[22px]' : 'left-0.5'
                }`} />
              </button>
            </div>
            <p className="text-sm text-gray-400">
              <span className={proxy.running ? 'text-green-400' : 'text-red-400'}>
                {proxy.running ? 'Operational' : 'Stopped'}
              </span>
              {proxy.running && <span> · Running for {formatUptime(proxy.uptime_seconds)}</span>}
            </p>
          </div>

          {/* Enforcement Mode */}
          <div className="bg-gray-800/50 border border-gray-700/50 rounded-xl p-5">
            <div className="flex items-center gap-3 mb-3">
              <div className="w-9 h-9 rounded-full bg-amber-500/15 flex items-center justify-center">
                <Eye className="w-4 h-4 text-amber-400" />
              </div>
              <h3 className="text-sm font-semibold text-white">Enforcement Mode</h3>
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => setMode('enforce')}
                className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  proxy.mode === 'enforce'
                    ? 'bg-red-500/20 text-red-400 border border-red-500/30'
                    : 'bg-gray-700/50 text-gray-400 border border-gray-600/30 hover:bg-gray-700'
                }`}
              >
                <Shield className="w-3.5 h-3.5 inline mr-1.5 -mt-0.5" />
                Enforce
              </button>
              <button
                onClick={() => setMode('monitor')}
                className={`flex-1 px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
                  proxy.mode === 'monitor'
                    ? 'bg-amber-500/20 text-amber-400 border border-amber-500/30'
                    : 'bg-gray-700/50 text-gray-400 border border-gray-600/30 hover:bg-gray-700'
                }`}
              >
                <Eye className="w-3.5 h-3.5 inline mr-1.5 -mt-0.5" />
                Monitor
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Area Chart — Request Volume Trend */}
        <div className="lg:col-span-2 bg-gray-800/50 border border-gray-700/50 rounded-xl p-5">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <TrendingUp className="w-4 h-4 text-blue-400" />
              <h3 className="text-sm font-semibold text-white">Request Volume</h3>
              {isDemo && (
                <span className="text-[10px] px-2 py-0.5 rounded-full bg-gray-700 text-gray-400">Demo Data</span>
              )}
            </div>
            <div className="flex gap-1">
              {(['24h', '7d', '30d'] as const).map(p => (
                <button
                  key={p}
                  onClick={() => setChartPeriod(p)}
                  className={`px-2.5 py-1 text-xs rounded-md transition-colors ${
                    chartPeriod === p
                      ? 'bg-blue-600 text-white'
                      : 'text-gray-400 hover:bg-gray-700'
                  }`}
                >
                  {p}
                </button>
              ))}
            </div>
          </div>
          <div className="h-[240px]">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={trendData} margin={{ top: 5, right: 5, bottom: 0, left: -20 }}>
                <defs>
                  <linearGradient id="gradTotal" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#3b82f6" stopOpacity={0.3} />
                    <stop offset="100%" stopColor="#3b82f6" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="gradBlocked" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#ef4444" stopOpacity={0.3} />
                    <stop offset="100%" stopColor="#ef4444" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" strokeOpacity={0.4} />
                <XAxis dataKey="time" tick={{ fontSize: 10, fill: '#6b7280' }} tickLine={false} axisLine={false} interval="preserveStartEnd" />
                <YAxis tick={{ fontSize: 10, fill: '#6b7280' }} tickLine={false} axisLine={false} />
                <Tooltip content={<CustomTooltip />} />
                <Area type="monotone" dataKey="total" name="Total" stroke="#3b82f6" strokeWidth={2} fill="url(#gradTotal)" />
                <Area type="monotone" dataKey="blocked" name="Blocked" stroke="#ef4444" strokeWidth={1.5} fill="url(#gradBlocked)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </div>

        {/* Pie Chart — Risk Distribution */}
        <div className="bg-gray-800/50 border border-gray-700/50 rounded-xl p-5">
          <div className="flex items-center gap-2 mb-4">
            <ShieldX className="w-4 h-4 text-amber-400" />
            <h3 className="text-sm font-semibold text-white">Risk Distribution</h3>
            {isDemo && (
              <span className="text-[10px] px-2 py-0.5 rounded-full bg-gray-700 text-gray-400">Demo</span>
            )}
          </div>
          <div className="h-[240px]">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={pieData}
                  cx="50%"
                  cy="45%"
                  innerRadius={55}
                  outerRadius={80}
                  paddingAngle={3}
                  dataKey="value"
                  stroke="none"
                >
                  {pieData.map((_, i) => (
                    <Cell key={i} fill={PIE_COLORS[i % PIE_COLORS.length]} />
                  ))}
                </Pie>
                <Legend
                  verticalAlign="bottom"
                  iconType="circle"
                  iconSize={8}
                  formatter={(value: string) => <span className="text-xs text-gray-400">{value}</span>}
                />
                <Tooltip
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: 8, fontSize: 12 }}
                  itemStyle={{ color: '#d1d5db' }}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>

      {/* Recent Events */}
      <div className="bg-gray-800/50 border border-gray-700/50 rounded-xl overflow-hidden">
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-700/50">
          <h3 className="text-sm font-semibold text-white">Recent Events</h3>
          <Link to="/events" className="text-xs text-blue-400 hover:text-blue-300 transition-colors">
            View All →
          </Link>
        </div>

        {events.length === 0 ? (
          <p className="text-gray-500 text-center py-16 text-sm">No events yet. Waiting for agent activity...</p>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-gray-700/50">
                  <th className="text-left px-5 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Risk</th>
                  <th className="text-left px-5 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Event Description</th>
                  <th className="text-left px-5 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">AI Agent</th>
                  <th className="text-left px-5 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Rule Tag</th>
                  <th className="text-right px-5 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Timestamp</th>
                </tr>
              </thead>
              <tbody>
                {events.slice(0, 20).map((ev, i) => {
                  const risk = riskConfig[ev.risk_level ?? ''] || riskConfig.LOW
                  return (
                    <tr key={ev.id || ev.action_id || i} className="border-b border-gray-700/30 hover:bg-gray-800/30 transition-colors">
                      <td className="px-5 py-3">
                        <div className="flex items-center gap-2">
                          <div className={`w-2 h-2 rounded-full ${risk.dot}`} />
                          <span className={`text-xs font-medium ${risk.text}`}>{risk.label}</span>
                        </div>
                      </td>
                      <td className="px-5 py-3">
                        <p className="text-gray-200 truncate max-w-xs">{ev.content || ev.explanation || 'Event'}</p>
                      </td>
                      <td className="px-5 py-3 text-gray-400">
                        {ev.agent || '—'}
                      </td>
                      <td className="px-5 py-3">
                        <div className="flex gap-1 flex-wrap">
                          {ev.matched_rules && ev.matched_rules.length > 0
                            ? ev.matched_rules.map((r, j) => <RuleTag key={j} tag={r} />)
                            : <span className="text-gray-600 text-xs">—</span>
                          }
                        </div>
                      </td>
                      <td className="px-5 py-3 text-right text-gray-500 text-xs whitespace-nowrap">
                        {ev.timestamp ? new Date(ev.timestamp).toLocaleTimeString() : ''}
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}
