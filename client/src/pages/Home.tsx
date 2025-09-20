import React, { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import './Home.css'

export default function Home() {
  const [room, setRoom] = useState('')
  const navigate = useNavigate()

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    const r = params.get('room')
    if (r) {
      navigate(`/room?room=${encodeURIComponent(r)}`)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  function connect() {
    if (room.trim() === '') {
      alert('ルーム名を入力してください')
      return
    }
    navigate(`/room?room=${encodeURIComponent(room)}`)
  }

  return (
    <div className="home-container">
      <h1>Meteor</h1>
      <div className="home-input-wrapper">
        <input
          className="input"
          value={room}
          onChange={(e) => setRoom(e.target.value)}
          placeholder="接続先のルーム名を入力"
          onKeyDown={(e) => {
            if (e.key === 'Enter') connect()
          }}
          autoFocus
          aria-label="ルーム名"
        />
        <button type="button" className="btn" onClick={connect} aria-label="接続ボタン">
          接続
        </button>
      </div>
    </div>
  )
}

