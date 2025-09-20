import React from 'react'
import { Outlet } from 'react-router-dom'
import Header from './components/Header'
import type { AppOutletContext, ConnectionStatus } from './types'

export default function App() {
    const [status, setStatus] = React.useState<ConnectionStatus>('disconnected')

    return (
        <div className="app-shell">
            <Header status={status} />
            <div className="app-content">
                <main className="app-main">
                    <Outlet context={{ setAppStatus: setStatus } satisfies AppOutletContext} />
                </main>
            </div>
        </div>
    )
}
