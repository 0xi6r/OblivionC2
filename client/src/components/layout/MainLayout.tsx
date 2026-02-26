import { Link, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  Target,
  Terminal,
  Settings,
  LogOut,
  Activity,
} from "lucide-react";
import { useConnectionStore } from "../../store/connectionStore";

export function MainLayout({ children }: { children: React.ReactNode }) {
  const location = useLocation();
  const { disconnect, serverAddress } = useConnectionStore();

  const navItems = [
    { path: "/dashboard", icon: LayoutDashboard, label: "Dashboard" },
    { path: "/campaigns", icon: Target, label: "Campaigns" },
    { path: "/tasks", icon: Terminal, label: "Tasks" },
    { path: "/settings", icon: Settings, label: "Settings" },
  ];

  return (
    <div className="flex h-screen bg-gray-900 text-gray-100">
      {/* Sidebar */}
      <aside className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col">
        <div className="p-6 border-b border-gray-700">
          <div className="flex items-center gap-3">
            <Activity className="w-8 h-8 text-red-500" />
            <h1 className="text-xl font-bold tracking-tight">OblivionC2</h1>
          </div>
          <p className="text-xs text-gray-400 mt-1">Red Team Operations</p>
        </div>

        <nav className="flex-1 p-4 space-y-1">
          {navItems.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname.startsWith(item.path);
            
            return (
              <Link
                key={item.path}
                to={item.path}
                className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-colors ${
                  isActive
                    ? "bg-red-600 text-white"
                    : "text-gray-400 hover:bg-gray-700 hover:text-white"
                }`}
              >
                <Icon className="w-5 h-5" />
                <span className="font-medium">{item.label}</span>
              </Link>
            );
          })}
        </nav>

        <div className="p-4 border-t border-gray-700">
          <div className="text-xs text-gray-400 mb-3">
            <p className="font-semibold">Connected to:</p>
            <p className="truncate">{serverAddress}</p>
          </div>
          <button
            onClick={disconnect}
            className="flex items-center gap-2 w-full px-4 py-2 text-sm text-red-400 hover:bg-red-900/30 rounded-lg transition-colors"
          >
            <LogOut className="w-4 h-4" />
            Disconnect
          </button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-auto bg-gray-900">
        <div className="p-8 max-w-7xl mx-auto">{children}</div>
      </main>
    </div>
  );
}