import { Link } from 'react-router-dom';

import { useAuth } from '../auth/useAuth';

const cards = [
  {
    icon: '🎪',
    title: 'Events',
    value: '3 drafts',
    color: 'bg-coral',
  },
  {
    icon: '💌',
    title: 'Invites',
    value: '24 queued',
    color: 'bg-teal',
  },
  {
    icon: '📎',
    title: 'Attachments',
    value: 'PDFs soon',
    color: 'bg-lilac',
  },
];

export function DashboardPage() {
  const auth = useAuth();
  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'friend';

  return (
    <section className="space-y-6">
      <div className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-teal">Dashboard</p>
        <h2 className="mt-2 text-4xl font-black leading-tight">
          Welcome back, {displayName}.
        </h2>
        <p className="mt-4 max-w-2xl text-lg text-slate-700">
          Your account session is active. Keep planning, check invites, or tune
          your host profile before the next gathering takes shape.
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
      <div className="grid gap-4 md:grid-cols-3">
        {cards.map((card) => (
          <article
            className="min-h-40 rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker"
            key={card.title}
          >
            <div
              className={`mb-4 inline-flex size-14 items-center justify-center rounded-lg border-2 border-ink ${card.color} text-3xl`}
            >
              {card.icon}
            </div>
            <h3 className="text-xl font-black">{card.title}</h3>
            <p className="mt-2 text-slate-700">{card.value}</p>
          </article>
        ))}
      </div>
    </section>
  );
}
