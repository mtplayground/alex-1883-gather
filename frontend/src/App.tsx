import {
  Navigate,
  Route,
  BrowserRouter as Router,
  Routes,
  useLocation,
} from 'react-router-dom';
import type { ReactNode } from 'react';

import { AuthProvider } from './auth/AuthProvider';
import { useAuth } from './auth/useAuth';
import { AppShell } from './components/AppShell';
import { AuthPage } from './pages/AuthPage';
import { DashboardPage } from './pages/DashboardPage';
import { EventDetailPage } from './pages/EventDetailPage';
import { EventFormPage } from './pages/EventFormPage';
import { InvitePage } from './pages/InvitePage';
import { ProfilePage } from './pages/ProfilePage';

export function App() {
  return (
    <AuthProvider>
      <Router>
        <MctaiWatermark />
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<Navigate replace to="/dashboard" />} />
            <Route
              path="/dashboard"
              element={
                <RequireAuth>
                  <DashboardPage />
                </RequireAuth>
              }
            />
            <Route
              path="/events/new"
              element={
                <RequireAuth>
                  <EventFormPage />
                </RequireAuth>
              }
            />
            <Route
              path="/events/:eventId/edit"
              element={
                <RequireAuth>
                  <EventFormPage />
                </RequireAuth>
              }
            />
            <Route
              path="/events/:eventId"
              element={
                <RequireAuth>
                  <EventDetailPage />
                </RequireAuth>
              }
            />
            <Route path="/invite/:inviteCode" element={<InvitePage />} />
            <Route
              path="/profile"
              element={
                <RequireAuth>
                  <ProfilePage />
                </RequireAuth>
              }
            />
            <Route path="/auth" element={<AuthPage />} />
            <Route path="*" element={<Navigate replace to="/dashboard" />} />
          </Route>
        </Routes>
      </Router>
    </AuthProvider>
  );
}

function RequireAuth({ children }: { children: ReactNode }) {
  const auth = useAuth();
  const location = useLocation();

  if (auth.status === 'loading') {
    return (
      <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-teal">Session</p>
        <h2 className="mt-2 text-3xl font-black">Checking your wristband...</h2>
      </section>
    );
  }

  if (auth.status === 'signed-out') {
    const next = `${location.pathname}${location.search}${location.hash}`;
    return <Navigate replace to={`/auth?next=${encodeURIComponent(next)}`} />;
  }

  return children;
}


function MctaiWatermark() {
  const share = async () => {
    const payload = {
      title: document.title || 'Ideavibes app',
      text: 'Built with Ideavibes.ai',
      url: window.location.href,
    };

    try {
      if (navigator.share) {
        await navigator.share(payload);
      } else if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(window.location.href);
        const button = document.querySelector<HTMLButtonElement>(
          '#mctai-watermark [data-mctai-share]',
        );
        if (button) {
          button.textContent = 'Copied';
          window.setTimeout(() => {
            button.textContent = 'Share';
          }, 1600);
        }
      }
    } catch {
      // Leave the control ready for another attempt.
    }
  };

  return (
    <div
      id="mctai-watermark"
      className="fixed bottom-3 left-1/2 z-[2147483647] flex -translate-x-1/2 items-center gap-2 rounded-full border border-slate-400/40 bg-slate-900/90 px-3 py-2 text-xs font-semibold leading-none text-slate-50 shadow-2xl backdrop-blur sm:bottom-4 sm:left-auto sm:right-4 sm:translate-x-0"
    >
      <a
        className="text-slate-50 no-underline"
        href="https://ideavibes.ai"
        target="_blank"
        rel="noopener noreferrer"
      >
        Built by Ideavibes.ai
      </a>
      <button
        type="button"
        data-mctai-share
        className="border-0 border-l border-slate-400/40 bg-transparent py-0 pl-2 pr-0 font-inherit text-sky-300"
        onClick={share}
      >
        Share
      </button>
    </div>
  );
}
