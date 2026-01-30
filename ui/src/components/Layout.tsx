import { NavLink, Outlet } from 'react-router-dom'
import { Shield, LayoutDashboard, ShieldCheck, ScrollText, Settings } from 'lucide-react'

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/rules', icon: ShieldCheck, label: 'Rules' },
  { to: '/events', icon: ScrollText, label: 'Events' },
  { to: '/settings', icon: Settings, label: 'Settings' },
]

export default function Layout({ connected }: { connected: boolean }) {
  return (
    <div className="min-h-screen bg-gray-900 text-gray-100 flex">
      {/* Sidebar */}
      <aside className="w-60 bg-gray-800 border-r border-gray-700 flex flex-col fixed h-full">
        <div className="p-4 flex items-center gap-3 border-b border-gray-700">
          <Shield className="w-7 h-7 text-blue-500" />
          <div>
            <h1 className="font-bold text-lg leading-tight">MoltBot Harness</h1>
            <p className="text-[11px] text-gray-500">Control Center</p>
          </div>
        </div>

        <nav className="flex-1 p-2 space-y-0.5">
          {navItems.map(item => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors ${
                  isActive
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-400 hover:bg-gray-700 hover:text-gray-200'
                }`
              }
            >
              <item.icon className="w-4 h-4" />
              {item.label}
            </NavLink>
          ))}
        </nav>

        <div className="p-3 border-t border-gray-700">
          <div className={`flex items-center gap-2 px-3 py-2 rounded-lg text-xs ${
            connected ? 'bg-green-900/30 text-green-400' : 'bg-red-900/30 text-red-400'
          }`}>
            <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
            {connected ? 'Connected' : 'Disconnected'}
          </div>
        </div>
      </aside>

      {/* Main */}
      <main className="flex-1 ml-60 p-6 overflow-auto min-h-screen">
        <Outlet />
      </main>
    </div>
  )
}
