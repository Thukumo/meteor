import React from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider, createBrowserRouter, redirect } from 'react-router-dom'
import App from './App'
import Home from './pages/Home'
import Room from './pages/Room'
import './index.css'
import './components/Header.css'

const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    errorElement: <div className="route-error">エラーが発生しました。</div>,
    children: [
      { index: true, element: <Home /> },
      {
        path: 'room',
        element: <Room />,
        loader: async ({ request }) => {
          const url = new URL(request.url)
          const room = url.searchParams.get('room')
          if (!room) return redirect('/')
          const api = `${url.origin}/api/v1/room/${encodeURIComponent(room)}/history`
          const res = await fetch(api, { signal: request.signal })
          if (!res.ok) throw new Response('Failed to load history', { status: res.status })
          const data = (await res.json()) as string[]
          return data
        },
      },
    ],
  },
])

createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
)
