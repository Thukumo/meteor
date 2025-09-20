import React, { useEffect, useRef, useState } from 'react'
import { useLoaderData, useOutletContext } from 'react-router-dom'
import type { AppOutletContext } from '../types'
import { useRoomParam } from '../hooks/useRoomParam'
import { useWebSocketRoom } from '../hooks/useWebSocketRoom'

export default function Room() {
    const { setAppStatus } = useOutletContext<AppOutletContext>()
    const room = useRoomParam() || ''
    const initialHistory = (useLoaderData() as string[]) ?? []
    const { history, wsState, loading, error, send, listRef, inputRef, attachAutoResize } =
        useWebSocketRoom(room, initialHistory, setAppStatus)

    const [atBottom, setAtBottom] = useState(true)
    const listDivRef = useRef<HTMLDivElement | null>(null)
    const sentinelRef = useRef<HTMLDivElement | null>(null)

    useEffect(() => {
        const rootEl = listDivRef.current
        const sentinelEl = sentinelRef.current
        if (!rootEl || !sentinelEl) return

        const computeBottomPx = () => {
            const h = rootEl.clientHeight || window.innerHeight || 0
            const ideal = Math.round(h * 0.1) // 10vh
            return Math.max(64, Math.min(ideal, 160))
        }

        let currentBottomPx = computeBottomPx()

        const makeObserver = () =>
            new IntersectionObserver(
                (entries) => {
                    const visible = entries.some((e) => e.isIntersecting)
                    setAtBottom(visible)
                },
                {
                    root: rootEl,
                    threshold: 0,
                    rootMargin: `0px 0px ${currentBottomPx}px 0px`,
                },
            )

        let observer = makeObserver()
        observer.observe(sentinelEl)

        const resizeObserver = new ResizeObserver(() => {
            const next = computeBottomPx()
            if (next !== currentBottomPx) {
                currentBottomPx = next
                observer.disconnect()
                observer = makeObserver()
                observer.observe(sentinelEl)
            }
        })
        resizeObserver.observe(rootEl)

        return () => {
            observer.disconnect()
            resizeObserver.disconnect()
        }
    }, [])

    function copyLink() {
        const origin = window.location.origin
        // 共有用リンクは日本語をそのまま見せたいので今のところエンコードしない(後で変更するかも)
        const url = `${origin}/?room=${room}`
        navigator.clipboard
            .writeText(url)
            .then(() => alert('ルームへのリンクをコピーしました'))
            .catch(() => alert('リンクのコピーに失敗しました'))
    }

    return (
        <div className="page-container room-page">
            <h1 className="page-title">ルーム {room}</h1>
            <div className="action-row">
                <button className="btn" onClick={copyLink}>
                    リンクをコピー
                </button>
                <button
                    className="btn"
                    type="button"
                    onClick={() => {
                        const url = `/stream/?room=${encodeURIComponent(room)}`
                        window.open(url, '_blank', 'noopener,noreferrer')
                    }}
                    aria-label="このルームで配信を開始"
                >
                    このルームで配信する（新しいタブ）
                </button>
            </div>
            <div className="room-list-wrap">
                <div
                    ref={(el) => {
                        listDivRef.current = el
                        listRef(el)
                    }}
                    className="room-list"
                >
                    {loading ? (
                        <div className="muted">読み込み中...</div>
                    ) : error ? (
                        <div className="error">エラー: {error}</div>
                    ) : history.length === 0 ? (
                        <div className="muted">履歴がありません</div>
                    ) : (
                        history.map((h, i) => (
                            <div key={i} className="room-item">
                                {h}
                            </div>
                        ))
                    )}
                    <div ref={sentinelRef} aria-hidden="true" className="list-sentinel" />
                </div>
                <button
                    type="button"
                    className={`btn scroll-to-bottom ${atBottom ? '' : 'is-visible'}`}
                    onClick={() => {
                        const el = listDivRef.current
                        if (!el) return
                        const prefersReduced = window.matchMedia(
                            '(prefers-reduced-motion: reduce)',
                        ).matches
                        if (prefersReduced) {
                            el.scrollTop = el.scrollHeight
                        } else {
                            el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' })
                        }
                    }}
                    aria-label="最下部へ移動"
                    title="最下部へ"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                    >
                        <polyline points="6 9 12 15 18 9" />
                    </svg>
                </button>
            </div>
            <div className="compose">
                <textarea
                    ref={(el) => {
                        inputRef(el)
                        attachAutoResize(el)
                    }}
                    className="input compose-textarea"
                    placeholder="メッセージを入力 (Shift+Enterで改行、Enterで送信)"
                    onKeyDown={(e) => {
                        if (e.key === 'Enter' && !e.shiftKey) {
                            e.preventDefault()
                            const target = e.currentTarget as HTMLTextAreaElement
                            send(target.value)
                        }
                    }}
                />
                <button
                    className="btn"
                    onClick={() => {
                        const el = document.querySelector<HTMLTextAreaElement>('.compose-textarea')
                        if (el) send(el.value)
                    }}
                    disabled={wsState !== 'connected'}
                >
                    送信
                </button>
            </div>
        </div>
    )
}
