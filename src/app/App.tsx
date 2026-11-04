import { useEffect, useRef, useState } from 'react';
import {
  NavLink,
  Navigate,
  Route,
  Routes,
  useLocation,
} from 'react-router-dom';
import {
  Archive,
  ArrowUpFromLine,
  ChevronLeft,
  ChevronRight,
  HardDrive,
  LayoutDashboard,
  LockKeyhole,
  Settings,
  ShieldCheck,
  Smartphone,
} from 'lucide-react';
import { Dashboard } from '../features/dashboard/Dashboard';
import { Archives } from '../features/archives/Archives';
import { ImportChat } from '../features/import/ImportChat';
import { Devices } from '../features/devices/Devices';
import { SettingsPage } from '../features/settings/SettingsPage';
import { Button } from '../shared/components/ui/button';
import { StorageProvider } from '../features/storage/StorageProvider';
const navigation = [
  { to: '/dashboard', label: 'Overview', icon: LayoutDashboard },
  { to: '/archives', label: 'Archives', icon: Archive },
  { to: '/import', label: 'Import Chat', icon: ArrowUpFromLine },
  { to: '/devices', label: 'Devices', icon: Smartphone },
];
export function App() {
  return (
    <StorageProvider>
      <AppShell />
    </StorageProvider>
  );
}

export function AppShell() {
  const [collapsed, setCollapsed] = useState(false);
  const location = useLocation();
  const main = useRef<HTMLElement>(null);
  const previousPath = useRef(location.pathname);
  useEffect(() => {
    const title =
      location.pathname === '/settings'
        ? 'Settings'
        : (navigation.find((item) => item.to === location.pathname)?.label ??
          'Overview');
    document.title = `${title} · ChatVault Desktop`;
    if (previousPath.current !== location.pathname) {
      main.current?.querySelector('h1')?.focus();
      main.current?.scrollTo?.(0, 0);
      previousPath.current = location.pathname;
    }
  }, [location.pathname]);
  return (
    <div className={`app-shell ${collapsed ? 'is-collapsed' : ''}`}>
      <a
        href="#main-content"
        className="skip-link"
        onClick={(event) => {
          event.preventDefault();
          main.current?.focus();
        }}
      >
        Skip to content
      </a>
      <aside className="sidebar" aria-label="Application sidebar">
        <div className="brand">
          <img src="/brand.svg" width="38" height="38" alt="" />
          <div className="sidebar-label">
            <strong>
              ChatVault<span>Desktop</span>
            </strong>
          </div>
        </div>
        <div className="sidebar-section-label sidebar-label">
          YOUR WORKSPACE
        </div>
        <nav aria-label="Main navigation">
          {navigation.map(({ to, label, icon: Icon }) => (
            <NavLink
              key={to}
              to={to}
              title={collapsed ? label : undefined}
              aria-label={label}
              className={({ isActive }) =>
                `nav-item ${isActive ? 'active' : ''}`
              }
            >
              <Icon size={19} aria-hidden="true" />
              <span className="sidebar-label">{label}</span>
            </NavLink>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="local-note sidebar-label">
            <ShieldCheck size={20} aria-hidden="true" />
            <strong>Yours. And only yours.</strong>
            <p>Your data stays on this computer. No accounts. No cloud.</p>
          </div>
          <NavLink
            to="/settings"
            aria-label="Settings"
            title={collapsed ? 'Settings' : undefined}
            className={({ isActive }) => `nav-item ${isActive ? 'active' : ''}`}
          >
            <Settings size={19} aria-hidden="true" />
            <span className="sidebar-label">Settings</span>
          </NavLink>
          <div className="sidebar-footer">
            <span className="sidebar-label">FOUNDATION · v0.1.0</span>
            <Button
              variant="ghost"
              size="icon"
              aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
              aria-expanded={!collapsed}
              onClick={() => setCollapsed((value) => !value)}
            >
              {collapsed ? (
                <ChevronRight size={17} />
              ) : (
                <ChevronLeft size={17} />
              )}
            </Button>
          </div>
        </div>
      </aside>
      <div className="workspace">
        <header className="topbar">
          <div className="breadcrumb">
            <HardDrive size={15} aria-hidden="true" />
            <span>Personal workspace</span>
            <span className="breadcrumb-divider">/</span>
            <strong>
              {location.pathname === '/settings'
                ? 'Settings'
                : (navigation.find((item) => item.to === location.pathname)
                    ?.label ?? 'Overview')}
            </strong>
          </div>
          <span className="local-status">
            <LockKeyhole size={13} aria-hidden="true" />
            Local only
          </span>
        </header>
        <main id="main-content" tabIndex={-1} ref={main}>
          <div className="page-content">
            <Routes>
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/archives" element={<Archives />} />
              <Route path="/import" element={<ImportChat />} />
              <Route path="/devices" element={<Devices />} />
              <Route path="/settings" element={<SettingsPage />} />
              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Routes>
          </div>
          <footer className="workspace-footer">
            <span>ChatVault Desktop</span>
            <span>
              <ShieldCheck size={12} aria-hidden="true" />
              Private by design. Local by default.
            </span>
          </footer>
        </main>
      </div>
    </div>
  );
}
