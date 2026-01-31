import { NavLink, Outlet } from 'react-router-dom'
import { Shield, LayoutDashboard, ShieldCheck, ScrollText, Settings } from 'lucide-react'

const navItems = [
  { to: '/', icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/rules', icon: ShieldCheck, label: 'Security Rules' },
  { to: '/events', icon: ScrollText, label: 'Activity Logs' },
  { to: '/settings', icon: Settings, label: 'System Settings' },
]

export default function Layout({ connected }: { connected: boolean }) {
  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 flex">
      <aside className="w-64 bg-gray-900 border-r border-gray-800 flex flex-col fixed h-full">
        <div className="p-5 flex items-center gap-3 border-b border-gray-800">
          <div className="w-9 h-9 bg-blue-600 rounded-lg flex items-center justify-center">
            <Shield className="w-5 h-5 text-white" />
          </div>
          <div>
            <h1 className="font-bold text-base leading-tight text-white">OpenClaw</h1>
            <p className="text-[11px] text-gray-500">Harness v1.2.0</p>
          </div>
        </div>

        <nav className="flex-1 p-3 space-y-1 mt-2">
          {navItems.map(item => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                  isActive
                    ? 'bg-blue-600 text-white'
                    : 'text-gray-400 hover:bg-gray-800 hover:text-gray-200'
                }`
              }
            >
              <item.icon className="w-[18px] h-[18px]" />
              {item.label}
            </NavLink>
          ))}
        </nav>

        <div className="p-4 border-t border-gray-800">
          <div className={`flex items-center justify-center gap-2 px-3 py-2 rounded-full text-xs font-medium ${
            connected
              ? 'bg-green-500/10 text-green-400 border border-green-500/20'
              : 'bg-red-500/10 text-red-400 border border-red-500/20'
          }`}>
            <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`} />
            {connected ? 'SYSTEM CONNECTED' : 'DISCONNECTED'}
          </div>
        </div>
      </aside>

      <main className="flex-1 ml-64 p-8 overflow-auto min-h-screen bg-gray-950">
        <Outlet />
      </main>
    </div>
  )
}
