import { NavLink, Outlet } from 'react-router-dom';

import { useAuth } from '../auth/useAuth';

const navigation = [
  { to: '/dashboard', label: 'Dashboard', icon: '☀️' },
  { to: '/events/summer-supper', label: 'Event', icon: '🎈' },
  { to: '/invite/teal-table', label: 'Invite', icon: '💌' },
  { to: '/profile', label: 'Profile', icon: '🌟' },
  { to: '/auth', label: 'Auth', icon: '🔐' },
];

export function AppShell() {
  const auth = useAuth();

  return (
    <div className="min-h-screen bg-[radial-gradient(circle_at_top_left,#ffd166_0_14%,transparent_15%),linear-gradient(135deg,#fffaf0_0%,#e9fff8_52%,#fff0ee_100%)] text-ink">
      <header className="border-b-4 border-ink bg-paper/90">
        <div className="mx-auto flex w-full max-w-7xl flex-col gap-4 px-5 py-5 md:flex-row md:items-center md:justify-between">
          <div>
            <p className="text-sm font-black uppercase text-coral">
              Event gathering workspace
            </p>
            <h1 className="text-3xl font-black leading-tight md:text-4xl">
              Playful planning shell
            </h1>
          </div>
          <a
            className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
            href={auth.buildLoginUrl('/dashboard')}
          >
            Sign in
          </a>
        </div>
        <nav
          className="mx-auto flex w-full max-w-7xl gap-2 overflow-x-auto px-5 pb-5"
          aria-label="Primary"
        >
          {navigation.map((item) => (
            <NavLink
              className={({ isActive }) =>
                [
                  'inline-flex min-h-11 shrink-0 items-center gap-2 rounded-lg border-2 border-ink px-4 py-2 text-sm font-black transition',
                  isActive
                    ? 'bg-teal text-white shadow-sticker'
                    : 'bg-white hover:bg-mint',
                ].join(' ')
              }
              key={item.to}
              to={item.to}
            >
              <span aria-hidden="true">{item.icon}</span>
              {item.label}
            </NavLink>
          ))}
        </nav>
      </header>
      <main className="mx-auto w-full max-w-7xl px-5 py-8">
        <Outlet />
      </main>
    </div>
  );
}
