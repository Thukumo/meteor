import React, { useEffect, useRef, useState } from 'react'
import { useLoaderData, useLocation, useOutletContext } from 'react-router-dom'
import type { AppOutletContext, ConnectionStatus } from '../types'

type Props = {
  setAppStatus?: (s: ConnectionStatus) => void
}

function useQuery() {
  return new URLSearchParams(useLocation().search)
}

export default function Room({ setAppStatus }: Props = {}) {
  const { setAppStatus: setStatusFromContext } = useOutletContext<AppOutletContext>()
  const query = useQuery()
  const room = query.get('room') || ''
  const initialHistory = useLoaderData() as string[]
  const [history, setHistory] = useState<string[]>(initialHistory ?? [])
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

    shouldReconnectRef.current = true

    const ac = new AbortController()

    // 初期履歴はloaderから供給済み
    setLoading(false)
    setTimeout(() => listRef.current?.scrollTo(0, listRef.current.scrollHeight), 50)

    const origin = window.location.origin
    const wsScheme = origin.startsWith('https') ? 'wss' : 'ws'
    const wsOrigin = origin.replace(/^https?/, wsScheme)

    function connectWs() {
      if (!shouldReconnectRef.current) return
  const set = setAppStatus || setStatusFromContext
  set && set('connecting')
      setWsState('connecting')
      const ws = new WebSocket(`${wsOrigin}/api/v1/room/${encodeURIComponent(room)}/ws`)
      wsRef.current = ws

      ws.addEventListener('open', () => {
        reconnectAttemptsRef.current = 0
        setError(null)
  set && set('connected')
        setWsState('connected')
      })

      ws.addEventListener('message', (ev) => {
        setHistory((h) => [...h, ev.data])
        setTimeout(() => listRef.current?.scrollTo(0, listRef.current.scrollHeight), 50)
      })

      ws.addEventListener('close', () => {
        if (!shouldReconnectRef.current) return
  set && set('disconnected')
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
      ac.abort()
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
    // 共有用リンクは日本語をそのまま見せたいので今のところエンコードしない(後で変更するかも)
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
