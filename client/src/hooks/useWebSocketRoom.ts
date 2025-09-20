import { useEffect, useRef, useState } from 'react'
import type { ConnectionStatus } from '../types'

export type RoomHookState = {
    history: string[]
    wsState: ConnectionStatus
    loading: boolean
    error: string | null
}

export type RoomHookApi = RoomHookState & {
    send: (text: string) => void
    attachAutoResize: (el: HTMLTextAreaElement | null) => void
    listRef: (el: HTMLDivElement | null) => void
    inputRef: (el: HTMLTextAreaElement | null) => void
}

export function useWebSocketRoom(
    room: string,
    initialHistory: string[],
    onStatus?: (s: ConnectionStatus) => void,
): RoomHookApi {
    const [history, setHistory] = useState<string[]>(initialHistory ?? [])
    const [loading, setLoading] = useState(true)
    const [error, setError] = useState<string | null>(null)
    const [wsState, setWsState] = useState<ConnectionStatus>('disconnected')
    const wsRef = useRef<WebSocket | null>(null)
    const listEl = useRef<HTMLDivElement | null>(null)
    const inputEl = useRef<HTMLTextAreaElement | null>(null)
    const reconnectAttemptsRef = useRef(0)
    const reconnectTimeoutRef = useRef<number | null>(null)
    const shouldReconnectRef = useRef(true)

    useEffect(() => {
        if (!room) return

        shouldReconnectRef.current = true

        // 初期履歴はloaderから供給済み
        setLoading(false)
        setTimeout(() => listEl.current?.scrollTo(0, listEl.current.scrollHeight), 50)

        const origin = window.location.origin
        const wsScheme = origin.startsWith('https') ? 'wss' : 'ws'
        const wsOrigin = origin.replace(/^https?/, wsScheme)

        function connectWs() {
            if (!shouldReconnectRef.current) return
            onStatus?.('connecting')
            setWsState('connecting')
            const ws = new WebSocket(`${wsOrigin}/api/v1/room/${encodeURIComponent(room)}/ws`)
            wsRef.current = ws

            ws.addEventListener('open', () => {
                reconnectAttemptsRef.current = 0
                setError(null)
                onStatus?.('connected')
                setWsState('connected')
            })

            ws.addEventListener('message', (ev: MessageEvent<string>) => {
                const data = typeof ev.data === 'string' ? ev.data : String(ev.data)
                setHistory((h) => [...h, data])
                setTimeout(() => {
                    const el = listEl.current
                    el?.scrollTo(0, el.scrollHeight)
                }, 50)
            })

            ws.addEventListener('close', () => {
                if (!shouldReconnectRef.current) return
                onStatus?.('disconnected')
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
                try {
                    ws.close()
                } catch {
                    // ignore
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
    }, [room, onStatus])

    function send(text: string) {
        const t = text.trim()
        if (!t) return
        if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
            wsRef.current.send(t)
            if (inputEl.current) {
                inputEl.current.value = ''
                inputEl.current.style.height = 'auto'
                inputEl.current.focus()
            }
        } else {
            alert('接続されていません')
        }
    }

    function attachAutoResize(el: HTMLTextAreaElement | null) {
        if (!el) return
        const handler = (e: Event) => {
            const t = e.currentTarget as HTMLTextAreaElement
            t.style.height = 'auto'
            t.style.height = `${t.scrollHeight}px`
        }
        el.addEventListener('input', handler, { passive: true })
        // 初期化時に一度実行
        el.dispatchEvent(new Event('input'))
        // 呼び出し側でクリーンアップが必要になったら、この関数を拡張する
    }

    return {
        history,
        wsState,
        loading,
        error,
        send,
        attachAutoResize,
        listRef: (el) => (listEl.current = el),
        inputRef: (el) => (inputEl.current = el),
    }
}
