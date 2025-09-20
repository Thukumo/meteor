import React, { useEffect, useRef, useState } from 'react'
import { useLocation } from 'react-router-dom'
import type { ConnectionStatus } from '../types'

type Props = {
  setAppRoom?: (r?: string) => void
  setAppStatus?: (s: ConnectionStatus) => void
}

function useQuery() {
  return new URLSearchParams(useLocation().search)
}

export default function Room({ setAppRoom, setAppStatus }: Props = {}) {
  const query = useQuery()
  const room = query.get('room') || ''
  const [history, setHistory] = useState<string[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [wsState, setWsState] = useState<ConnectionStatus>('disconnected')
  const wsRef = useRef<WebSocket | null>(null)
  const inputRef = useRef<HTMLTextAreaElement | null>(null)
  const listRef = useRef<HTMLDivElement | null>(null)
  const reconnectAttemptsRef = useRef(0)
  const reconnectTimeoutRef = useRef<number | null>(null)
  const shouldReconnectRef = useRef(true)

  useEffect(() => {
    if (!room) return

    // tell app which room we're in
    setAppRoom && setAppRoom(room)

    shouldReconnectRef.current = true

    async function load() {
      setLoading(true)
      setError(null)
      try {
        const origin = window.location.origin
        const res = await fetch(`${origin}/api/v1/room/${encodeURIComponent(room)}/history`)
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        const data: string[] = await res.json()
        setHistory(data)
        setLoading(false)
        setTimeout(() => listRef.current?.scrollTo(0, listRef.current.scrollHeight), 50)
      } catch (e: unknown) {
        setLoading(false)
        setError(e instanceof Error ? e.message : 'Unknown error')
      }
    }
    load()

    const origin = window.location.origin
    const wsScheme = origin.startsWith('https') ? 'wss' : 'ws'
    const wsOrigin = origin.replace(/^https?/, wsScheme)

    function connectWs() {
      if (!shouldReconnectRef.current) return
      setAppStatus && setAppStatus('connecting')
      setWsState('connecting')
      const ws = new WebSocket(`${wsOrigin}/api/v1/room/${encodeURIComponent(room)}/ws`)
      wsRef.current = ws

      ws.addEventListener('open', () => {
        reconnectAttemptsRef.current = 0
        setError(null)
        setAppStatus && setAppStatus('connected')
        setWsState('connected')
      })

      ws.addEventListener('message', (ev) => {
        setHistory((h) => [...h, ev.data])
        setTimeout(() => listRef.current?.scrollTo(0, listRef.current.scrollHeight), 50)
      })

      ws.addEventListener('close', () => {
        if (!shouldReconnectRef.current) return
        setAppStatus && setAppStatus('disconnected')
        setWsState('disconnected')
        // exponential backoff with jitter
        reconnectAttemptsRef.current += 1
        const maxAttempts = 10
        if (reconnectAttemptsRef.current > maxAttempts) {
          setError('接続を再試行しましたが、復旧しませんでした。')
          return
        }
        const base = 1000
        const expo = Math.min(base * Math.pow(2, reconnectAttemptsRef.current - 1), 30000)
        const jitter = Math.random() * 300
        const delay = Math.floor(expo + jitter)
        if (reconnectTimeoutRef.current) window.clearTimeout(reconnectTimeoutRef.current)
        reconnectTimeoutRef.current = window.setTimeout(() => {
          connectWs()
        }, delay)
      })

      ws.addEventListener('error', () => {
        // close will trigger reconnect logic
        try {
          ws.close()
        } catch {
          // Ignore errors when closing WebSocket
        }
      })
    }

    connectWs()

    return () => {
      shouldReconnectRef.current = false
      if (reconnectTimeoutRef.current) window.clearTimeout(reconnectTimeoutRef.current)
      wsRef.current?.close()
      wsRef.current = null
    }
  }, [room])

  function sendMessage() {
    const text = inputRef.current?.value.trim() || ''
    if (!text) return
    if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
      wsRef.current.send(text)
      if (inputRef.current) {
        inputRef.current.value = ''
        inputRef.current.style.height = 'auto'
        inputRef.current.focus()
      }
    } else {
      alert('接続されていません')
    }
  }

  function copyLink() {
    const origin = window.location.origin
    // Copy a link that points to the root with a query parameter.
    // Many simple servers return 404 for client-side routes like /room,
    // so use /?room=... which `Home` handles and navigates into the SPA.
    // Use raw room value so clipboard contains original characters (no percent-encoding).
    // Note: if room contains characters like spaces, the resulting URL may not be valid
    // as a single token; this matches the user's request to avoid percent-encoding.
    const url = `${origin}/?room=${room}`
    navigator.clipboard.writeText(url).then(() => alert('ルームへのリンクをコピーしました'))
  }

  return (
    <div className="page-container">
      <h1 className="page-title">ルーム {room}</h1>
      <button className="btn" onClick={copyLink}>リンクをコピー</button>
      <div ref={listRef} className="room-list">
        {loading ? (
          <div className="muted">読み込み中...</div>
        ) : error ? (
          <div className="error">エラー: {error}</div>
        ) : history.length === 0 ? (
          <div className="muted">履歴がありません</div>
        ) : (
          history.map((h, i) => <div key={i} className="room-item">{h}</div>)
        )}
      </div>
      <div className="compose">
        <textarea
          ref={inputRef}
          className="input compose-textarea"
          placeholder="メッセージを入力 (Shift+Enterで改行、Enterで送信)"
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault()
              sendMessage()
            }
          }}
          onInput={(e) => {
            const t = e.currentTarget as HTMLTextAreaElement
            t.style.height = 'auto'
            t.style.height = `${t.scrollHeight}px`
          }}
        />
        <button className="btn" onClick={sendMessage} disabled={wsState !== 'connected'}>送信</button>
      </div>
    </div>
  )
}
