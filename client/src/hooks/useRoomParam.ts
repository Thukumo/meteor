import { useLocation } from 'react-router-dom'

export function useRoomParam(): string | undefined {
    const search = useLocation().search
    const params = new URLSearchParams(search)
    const r = params.get('room')?.trim()
    return r ? r : undefined
}
