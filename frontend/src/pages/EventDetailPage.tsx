import { useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import {
  apiClient,
  type DashboardEventSummary,
  type EventAttachmentRecord,
  type EventRecord,
} from '../api/client';
import { useAuth } from '../auth/useAuth';

type EventDetailState =
  | {
      status: 'loading';
      event: null;
      attachments: EventAttachmentRecord[];
      dashboardEvent: null;
      error: null;
    }
  | {
      status: 'ready';
      event: EventRecord;
      attachments: EventAttachmentRecord[];
      dashboardEvent: DashboardEventSummary | null;
      error: null;
    }
  | {
      status: 'error';
      event: null;
      attachments: EventAttachmentRecord[];
      dashboardEvent: null;
      error: string;
    };

export function EventDetailPage() {
  const { eventId } = useParams();
  const auth = useAuth();
  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'You';
  const [detail, setDetail] = useState<EventDetailState>({
    status: 'loading',
    event: null,
    attachments: [],
    dashboardEvent: null,
    error: null,
  });
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    if (!eventId) {
      return () => {
        cancelled = true;
      };
    }

    Promise.all([
      apiClient.event(eventId),
      apiClient.eventAttachments(eventId),
      apiClient.dashboardEvents(100).catch(() => ({ events: [] })),
    ])
      .then(([event, attachmentResponse, dashboardResponse]) => {
        if (cancelled) {
          return;
        }

        setDetail({
          status: 'ready',
          event,
          attachments: attachmentResponse.attachments,
          dashboardEvent:
            dashboardResponse.events.find((item) => item.id === event.id) ??
            null,
          error: null,
        });
      })
      .catch((error: unknown) => {
        if (cancelled) {
          return;
        }

        setDetail({
          status: 'error',
          event: null,
          attachments: [],
          dashboardEvent: null,
          error:
            error instanceof Error
              ? error.message
              : 'We could not load this event.',
        });
      });

    return () => {
      cancelled = true;
    };
  }, [eventId]);

  const activityItems = useMemo(() => {
    if (detail.status !== 'ready') {
      return [];
    }

    return [
      {
        label: 'Updated',
        value: formatShortDate(detail.event.updated_at),
      },
      {
        label: 'Created',
        value: formatShortDate(detail.event.created_at),
      },
      {
        label: 'Files',
        value: String(detail.attachments.length),
      },
    ];
  }, [detail]);

  if (!eventId) {
    return (
      <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-coral">Event detail</p>
        <h2 className="mt-2 text-3xl font-black">This event did not load.</h2>
        <p className="mt-3 text-slate-700">Event not found.</p>
        <Link
          className="mt-5 inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
          to="/dashboard"
        >
          Back to dashboard
        </Link>
      </section>
    );
  }

  if (detail.status === 'loading') {
    return <EventDetailLoading />;
  }

  if (detail.status === 'error') {
    return (
      <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-coral">Event detail</p>
        <h2 className="mt-2 text-3xl font-black">This event did not load.</h2>
        <p className="mt-3 text-slate-700">{detail.error}</p>
        <Link
          className="mt-5 inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
          to="/dashboard"
        >
          Back to dashboard
        </Link>
      </section>
    );
  }

  const coverUrl = detail.dashboardEvent?.cover_image_url ?? null;
  const relationship = relationshipLabel(
    detail.dashboardEvent?.relationship ?? 'invited',
  );
  const schedule = formatEventDateTime(
    detail.event.starts_at,
    detail.event.timezone,
  );

  async function downloadAttachment(attachment: EventAttachmentRecord) {
    if (!eventId) {
      return;
    }

    setDownloadingId(attachment.id);
    setDownloadError(null);

    try {
      const response = await apiClient.eventAttachmentDownload(
        eventId,
        attachment.id,
      );
      const anchor = document.createElement('a');
      anchor.href = response.access_url;
      anchor.download = attachment.filename;
      anchor.rel = 'noreferrer';
      anchor.target = '_blank';
      anchor.click();
    } catch (error) {
      setDownloadError(
        error instanceof Error
          ? error.message
          : 'We could not prepare that download.',
      );
    } finally {
      setDownloadingId(null);
    }
  }

  return (
    <section className="space-y-6">
      <div className="grid gap-6 xl:grid-cols-[1.45fr_0.55fr]">
        <article className="overflow-hidden rounded-lg border-4 border-ink bg-white shadow-sticker">
          <div className="relative min-h-[23rem] bg-mint md:min-h-[31rem]">
            {coverUrl ? (
              <img
                alt=""
                className="absolute inset-0 h-full w-full object-cover"
                src={coverUrl}
              />
            ) : (
              <div className="absolute inset-0 flex items-center justify-center bg-[linear-gradient(135deg,#8bd3c7_0%,#ffd166_48%,#ff7a70_100%)] text-7xl">
                🎪
              </div>
            )}
            <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-ink/80 via-ink/45 to-transparent p-5 text-white md:p-7">
              <p className="inline-flex rounded-lg border-2 border-white bg-teal px-3 py-1 text-xs font-black uppercase shadow-sticker">
                {relationship}
              </p>
              <h2 className="mt-3 max-w-4xl text-4xl font-black leading-tight md:text-6xl">
                {detail.event.title}
              </h2>
            </div>
          </div>
        </article>

        <aside className="space-y-5">
          <Link
            className="flex min-h-12 items-center justify-center rounded-lg border-4 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
            to={`/events/${detail.event.id}/edit`}
          >
            Edit event
          </Link>
          <section className="rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker">
            <p className="text-sm font-black uppercase text-coral">When</p>
            <h3 className="mt-2 text-2xl font-black leading-tight">
              {schedule}
            </h3>
            {detail.event.timezone ? (
              <p className="mt-2 text-sm font-bold text-slate-700">
                {detail.event.timezone}
              </p>
            ) : null}
          </section>

          <section className="rounded-lg border-4 border-ink bg-white p-5 shadow-sticker">
            <p className="text-sm font-black uppercase text-teal">RSVPs</p>
            <ul className="mt-4 space-y-3">
              <li className="flex items-center justify-between gap-3 rounded-lg border-2 border-ink bg-mint px-3 py-2">
                <span className="font-black">{displayName}</span>
                <span className="rounded-lg bg-white px-3 py-1 text-sm font-black">
                  {relationship}
                </span>
              </li>
            </ul>
          </section>
        </aside>
      </div>

      <div className="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
          <p className="text-sm font-black uppercase text-teal">Details</p>
          {detail.event.description ? (
            <p className="mt-3 whitespace-pre-line text-lg leading-relaxed text-slate-700">
              {detail.event.description}
            </p>
          ) : (
            <p className="mt-3 text-lg text-slate-700">
              Details are still taking shape.
            </p>
          )}
        </section>

        <section className="rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <p className="text-sm font-black uppercase text-coral">
                Attachments
              </p>
              <h3 className="mt-1 text-2xl font-black">
                {detail.attachments.length} PDF
                {detail.attachments.length === 1 ? '' : 's'}
              </h3>
            </div>
            {downloadError ? (
              <p className="max-w-sm text-sm font-bold text-coral">
                {downloadError}
              </p>
            ) : null}
          </div>

          {detail.attachments.length > 0 ? (
            <ul className="mt-5 space-y-3">
              {detail.attachments.map((attachment) => (
                <li
                  className="flex flex-col gap-3 rounded-lg border-2 border-ink bg-white p-4 sm:flex-row sm:items-center sm:justify-between"
                  key={attachment.id}
                >
                  <div className="min-w-0">
                    <p className="truncate font-black">
                      {attachment.filename}
                    </p>
                    <p className="mt-1 text-sm text-slate-600">
                      {formatBytes(attachment.byte_size)}
                    </p>
                  </div>
                  <button
                    className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
                    disabled={downloadingId === attachment.id}
                    onClick={() => void downloadAttachment(attachment)}
                    type="button"
                  >
                    {downloadingId === attachment.id ? 'Preparing' : 'Download'}
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <div className="mt-5 rounded-lg border-2 border-ink bg-white p-4">
              <p className="font-black">No PDFs yet</p>
              <p className="mt-1 text-slate-700">
                Menus, schedules, and notes will appear here.
              </p>
            </div>
          )}
        </section>
      </div>

      <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-teal">Activity</p>
        <div className="mt-4 grid gap-3 md:grid-cols-3">
          {activityItems.map((item) => (
            <div
              className="rounded-lg border-2 border-ink bg-mint p-4"
              key={item.label}
            >
              <p className="text-xs font-black uppercase text-slate-600">
                {item.label}
              </p>
              <p className="mt-2 text-xl font-black">{item.value}</p>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}

function EventDetailLoading() {
  return (
    <section className="space-y-6">
      <div className="grid gap-6 xl:grid-cols-[1.45fr_0.55fr]">
        <div className="min-h-[31rem] animate-pulse rounded-lg border-4 border-ink bg-mint shadow-sticker" />
        <div className="space-y-5">
          <div className="h-40 animate-pulse rounded-lg border-4 border-ink bg-paper shadow-sticker" />
          <div className="h-44 animate-pulse rounded-lg border-4 border-ink bg-white shadow-sticker" />
        </div>
      </div>
      <div className="h-52 animate-pulse rounded-lg border-4 border-ink bg-white shadow-sticker" />
    </section>
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

function formatEventDateTime(startsAt: string, timezone: string | null) {
  const date = new Date(startsAt);
  const options: Intl.DateTimeFormatOptions = {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    year: 'numeric',
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

function formatShortDate(value: string) {
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  }).format(new Date(value));
}

function formatBytes(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[unitIndex]}`;
}
