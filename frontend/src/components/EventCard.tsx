import { Link } from 'react-router-dom';

import type { DashboardEventSummary } from '../api/client';

type EventCardProps = {
  event: DashboardEventSummary;
};

export function EventCard({ event }: EventCardProps) {
  const schedule = formatEventTime(event.starts_at, event.timezone);
  const relationship = relationshipLabel(event.relationship);

  return (
    <article className="group grid min-h-[25rem] overflow-hidden rounded-lg border-4 border-ink bg-white shadow-sticker transition hover:-translate-y-1 md:grid-rows-[15rem_1fr]">
      <Link
        aria-label={`Open ${event.title}`}
        className="relative block min-h-60 overflow-hidden bg-mint md:min-h-0"
        to={`/events/${event.id}`}
      >
        {event.cover_image_url ? (
          <img
            alt=""
            className="h-full w-full object-cover transition duration-300 group-hover:scale-105"
            src={event.cover_image_url}
          />
        ) : (
          <div className="flex h-full min-h-60 w-full items-center justify-center bg-[linear-gradient(135deg,#8bd3c7_0%,#ffd166_52%,#ff7a70_100%)] p-6 text-center md:min-h-0">
            <span className="text-6xl" aria-hidden="true">
              🎪
            </span>
          </div>
        )}
        <span className="absolute left-4 top-4 rounded-lg border-2 border-ink bg-paper px-3 py-1 text-xs font-black uppercase shadow-sticker">
          {relationship}
        </span>
      </Link>
      <div className="flex min-h-0 flex-col p-5">
        <p className="text-sm font-black uppercase text-coral">{schedule}</p>
        <h3 className="mt-2 text-2xl font-black leading-tight">
          <Link className="hover:text-teal" to={`/events/${event.id}`}>
            {event.title}
          </Link>
        </h3>
        {event.description ? (
          <p className="mt-3 line-clamp-3 text-slate-700">{event.description}</p>
        ) : (
          <p className="mt-3 text-slate-600">Details are still taking shape.</p>
        )}
        <div className="mt-auto pt-5">
          <Link
            className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
            to={`/events/${event.id}`}
          >
            Open event
          </Link>
        </div>
      </div>
    </article>
  );
}

function relationshipLabel(relationship: string) {
  switch (relationship) {
    case 'organizer':
      return 'Hosting';
    case 'joined':
      return 'Going';
    case 'invited':
      return 'Invited';
    default:
      return relationship;
  }
}

function formatEventTime(startsAt: string, timezone: string | null) {
  const date = new Date(startsAt);
  const options: Intl.DateTimeFormatOptions = {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  };

  try {
    return new Intl.DateTimeFormat('en-US', {
      ...options,
      ...(timezone ? { timeZone: timezone } : {}),
    }).format(date);
  } catch {
    return new Intl.DateTimeFormat('en-US', options).format(date);
  }
}
