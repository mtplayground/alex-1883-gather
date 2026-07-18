import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';

import {
  apiClient,
  type DashboardEventSummary,
} from '../api/client';
import { useAuth } from '../auth/useAuth';
import { EventCard } from '../components/EventCard';

type DashboardState =
  | { status: 'loading'; events: DashboardEventSummary[]; error: null }
  | { status: 'ready'; events: DashboardEventSummary[]; error: null }
  | { status: 'error'; events: DashboardEventSummary[]; error: string };

export function DashboardPage() {
  const auth = useAuth();
  const [dashboard, setDashboard] = useState<DashboardState>({
    status: 'loading',
    events: [],
    error: null,
  });
  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'friend';

  useEffect(() => {
    let cancelled = false;

    apiClient
      .dashboardEvents()
      .then((response) => {
        if (cancelled) {
          return;
        }

        setDashboard({
          status: 'ready',
          events: response.events,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setDashboard({
          status: 'error',
          events: [],
          error:
            error instanceof Error
              ? error.message
              : 'We could not load your events.',
        });
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-teal">
          Upcoming gatherings
        </p>
        <h2 className="mt-2 text-4xl font-black leading-tight">
          Welcome back, {displayName}
        </h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Your next plans are ready to scan: hosted events, accepted gatherings,
          and fresh invitations in one place.
        </p>
        {auth.user && !auth.user.emailVerified ? (
          <Link
            className="mt-5 inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
            to="/auth?mode=verify"
          >
            Check email verification
          </Link>
        ) : null}
      </div>

      {dashboard.status === 'loading' ? <DashboardLoading /> : null}

      {dashboard.status === 'error' ? (
        <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
          <p className="text-sm font-black uppercase text-coral">Try again</p>
          <h3 className="mt-2 text-2xl font-black">
            The event list did not load.
          </h3>
          <p className="mt-2 text-slate-700">{dashboard.error}</p>
        </div>
      ) : null}

      {dashboard.status === 'ready' && dashboard.events.length === 0 ? (
        <div className="grid gap-5 rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker md:grid-cols-[1fr_12rem] md:items-center">
          <div>
            <p className="text-sm font-black uppercase text-coral">
              No upcoming events
            </p>
            <h3 className="mt-2 text-3xl font-black">
              Your calendar has room for something good.
            </h3>
            <p className="mt-3 text-slate-700">
              Hosted events and invitations will appear here once they are on
              the schedule.
            </p>
          </div>
          <div className="flex aspect-square items-center justify-center rounded-lg border-4 border-ink bg-mint text-6xl shadow-sticker">
            🎈
          </div>
        </div>
      ) : null}

      {dashboard.status === 'ready' && dashboard.events.length > 0 ? (
        <div className="grid gap-5 lg:grid-cols-2 xl:grid-cols-3">
          {dashboard.events.map((event) => (
            <EventCard event={event} key={event.id} />
          ))}
        </div>
      ) : null}
    </section>
  );
}

function DashboardLoading() {
  return (
    <div className="grid gap-5 lg:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: 6 }, (_, index) => (
        <div
          className="min-h-[25rem] animate-pulse overflow-hidden rounded-lg border-4 border-ink bg-white shadow-sticker"
          key={index}
        >
          <div className="h-60 bg-mint" />
          <div className="space-y-4 p-5">
            <div className="h-4 w-32 rounded-lg bg-slate-200" />
            <div className="h-8 w-4/5 rounded-lg bg-slate-200" />
            <div className="h-4 w-full rounded-lg bg-slate-200" />
            <div className="h-4 w-2/3 rounded-lg bg-slate-200" />
          </div>
        </div>
      ))}
    </div>
  );
}
