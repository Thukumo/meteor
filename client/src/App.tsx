import React from 'react'
import { Routes, Route } from 'react-router-dom'
import Home from './pages/Home'
import Room from './pages/Room'
import Header from './components/Header'
import type { ConnectionStatus } from './types'

export default function App() {
  const [currentRoom, setCurrentRoom] = React.useState<string | undefined>(undefined)
  const [status, setStatus] = React.useState<ConnectionStatus>('disconnected')

  return (
    <div className="app-shell">
      <Header room={currentRoom} status={status} />
      <div className="app-content">
        <main className="app-main">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route
              path="/room"
              element={<Room setAppRoom={setCurrentRoom} setAppStatus={setStatus} />}
            />
          </Routes>
        </main>
      </div>
    </div>
  )
}
