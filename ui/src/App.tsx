import { BrowserRouter, Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import Dashboard from './pages/Dashboard'
import Rules from './pages/Rules'
import Events from './pages/Events'
import Settings from './pages/Settings'
import { useWebSocket } from './hooks/useWebSocket'

function App() {
  const { connected, events } = useWebSocket()

  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout connected={connected} />}>
          <Route path="/" element={<Dashboard events={events} />} />
          <Route path="/rules" element={<Rules />} />
          <Route path="/events" element={<Events liveEvents={events} />} />
          <Route path="/settings" element={<Settings />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}

export default App
