// Common type definitions used across the application

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected'

export type RoomMessage = string

export type AppState = {
    currentRoom?: string
    status: ConnectionStatus
}

// Outlet context for child routes to update global status
export type AppOutletContext = {
    setAppStatus: (_status: ConnectionStatus) => void
}
