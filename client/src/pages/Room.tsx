import React from 'react'
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
        <div className="page-container">
            <h1 className="page-title">ルーム {room}</h1>
            <button className="btn" onClick={copyLink}>
                リンクをコピー
            </button>
            <div className="mt-8">
                <button
                    className="btn"
                    type="button"
                    onClick={() => {
                        const url = `/stream/?room=${encodeURIComponent(room)}`
                        window.open(url, '_blank', 'noopener,noreferrer')
                    }}
                    aria-label="このルームで配信を開始"
                >
                    このルームで配信する（新しいタブが開きます）
                </button>
            </div>
            <div ref={listRef} className="room-list">
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
