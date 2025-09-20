import React from 'react'
import { Link, useLocation } from 'react-router-dom'
import './Header.css'
import type { ConnectionStatus } from '../types'

type Props = {
  appName?: string
  status?: ConnectionStatus
}

const statusLabels: Record<ConnectionStatus, string> = {
  connected: '接続済み',
  connecting: '再接続中',
  disconnected: '切断'
} as const

export default function Header({ appName = 'Meteor', status = 'disconnected' }: Props) {
  const location = useLocation()
  let room: string | undefined
  if (location.pathname === '/room') {
    const params = new URLSearchParams(location.search)
    const r = params.get('room')
    room = r && r.length > 0 ? r : undefined
  }
  return (
    <header className="app-header">
      <div className="app-header-left">
        <Link to="/" aria-label="ホームへ移動" style={{ color: 'inherit', textDecoration: 'none' }}>
          {appName}
        </Link>
      </div>
      <div className="app-header-center">{room ? `ルーム ${room}` : ''}</div>
      <div className={`app-header-right status-${status}`} aria-live="polite">
        <span className="status-dot" />
        <span className="status-text">{statusLabels[status]}</span>
      </div>
    </header>
  )
}
