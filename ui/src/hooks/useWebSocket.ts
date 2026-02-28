import { useState, useEffect, useRef } from 'react'
import { WS_BASE } from '../lib/api'

export interface WsEvent {
  type: 'action' | 'analysis' | 'status'
  id?: string
  timestamp?: string
  agent?: string
  action_type?: string
  content?: string
  target?: string
  risk_level?: string
  matched_rules?: string[]
  action_id?: string
  recommendation?: string
  explanation?: string
  connected?: boolean
}

export function useWebSocket() {
  const [connected, setConnected] = useState(false)
  const [events, setEvents] = useState<WsEvent[]>([])
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimerRef = useRef<number | null>(null)

  useEffect(() => {
    let disposed = false

    const connect = () => {
      if (disposed) return
      const ws = new WebSocket(`${WS_BASE}/ws/events`)
      wsRef.current = ws

      ws.onopen = () => setConnected(true)
      ws.onmessage = (e) => {
        try {
          const data = JSON.parse(e.data) as WsEvent
          if (data.type === 'action' || data.type === 'analysis') {
            setEvents(prev => [data, ...prev].slice(0, 100))
          }
        } catch (err) {
          console.debug('websocket parse failed', err)
        }
      }
      ws.onclose = () => {
        setConnected(false)
        if (disposed) return
        reconnectTimerRef.current = window.setTimeout(connect, 3000)
      }
      ws.onerror = () => ws.close()
    }

    connect()

    return () => {
      disposed = true
      wsRef.current?.close()
      if (reconnectTimerRef.current != null) {
        window.clearTimeout(reconnectTimerRef.current)
      }
    }
  }, [])

  return { connected, events }
}
