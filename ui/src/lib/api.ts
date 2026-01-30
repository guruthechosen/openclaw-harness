const API_BASE = import.meta.env.DEV ? 'http://localhost:8380' : ''
export const WS_BASE = import.meta.env.DEV ? 'ws://localhost:8380' : `ws://${window.location.host}`

async function api<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  if (!res.ok) throw new Error(`API error: ${res.status}`)
  if (res.status === 204) return undefined as T
  return res.json()
}

// Types
export interface Stats {
  total_events: number
  critical_count: number
  warning_count: number
  info_count: number
  today_events: number
  rules_count: number
  blocked_count: number
  passed_count: number
}

export interface ProviderStats {
  provider: string
  request_count: number
}

export interface RuleData {
  name: string
  description: string
  pattern: string
  risk_level: string
  action: string
  enabled: boolean
  is_preset: boolean
}

export interface ProxyStatus {
  running: boolean
  mode: string
  listen: string
  target: string
  uptime_seconds: number
}

export interface ProviderData {
  name: string
  enabled: boolean
  target_url: string
}

export interface AlertConfig {
  telegram_enabled: boolean
  telegram_bot_token: string | null
  telegram_chat_id: string | null
  slack_enabled: boolean
  slack_webhook: string | null
  discord_enabled: boolean
  discord_webhook: string | null
  notify_on_critical: boolean
  notify_on_warning: boolean
  notify_on_info: boolean
}

export interface EventData {
  id: string
  timestamp: string
  agent: string
  action_type: string
  content: string
  target?: string
  risk_level?: string
  matched_rules: string[]
  provider?: string
  status?: string
}

export interface TestRuleResult {
  matches: boolean
  matched_text: string | null
}

// API calls
export const getStats = () => api<Stats>('/api/stats')
export const getStatsByProvider = () => api<ProviderStats[]>('/api/stats/by-provider')
export const getProxyStatus = () => api<ProxyStatus>('/api/proxy/status')
export const updateProxyConfig = (data: { mode?: string; enabled?: boolean }) =>
  api<ProxyStatus>('/api/proxy/config', { method: 'PUT', body: JSON.stringify(data) })
export const getRules = () => api<RuleData[]>('/api/rules')
export const createRule = (data: { name: string; description: string; pattern: string; risk_level: string; action: string }) =>
  api<RuleData>('/api/rules', { method: 'POST', body: JSON.stringify(data) })
export const updateRule = (name: string, data: { description?: string; pattern?: string; risk_level?: string; action?: string; enabled?: boolean }) =>
  api<RuleData>(`/api/rules/${encodeURIComponent(name)}`, { method: 'PUT', body: JSON.stringify(data) })
export const deleteRule = (name: string) =>
  api<void>(`/api/rules/${encodeURIComponent(name)}`, { method: 'DELETE' })
export const testRule = (pattern: string, input: string) =>
  api<TestRuleResult>('/api/rules/test', { method: 'POST', body: JSON.stringify({ pattern, input }) })
export const getProviders = () => api<ProviderData[]>('/api/providers')
export const getAlertConfig = () => api<AlertConfig>('/api/alerts/config')
export const updateAlertConfig = (data: AlertConfig) =>
  api<void>('/api/alerts/config', { method: 'PUT', body: JSON.stringify(data) })
export const getEvents = (params?: { limit?: number; offset?: number; status?: string; provider?: string }) => {
  const qs = new URLSearchParams()
  if (params?.limit) qs.set('limit', String(params.limit))
  if (params?.offset) qs.set('offset', String(params.offset))
  if (params?.status) qs.set('status', params.status)
  if (params?.provider) qs.set('provider', params.provider)
  return api<{ events: EventData[]; total: number }>(`/api/events?${qs}`)
}
export const getRecentEvents = () => api<EventData[]>('/api/events/recent')
