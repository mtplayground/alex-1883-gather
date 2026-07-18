import { useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import {
  apiClient,
  type DashboardEventSummary,
  type EventAttachmentRecord,
  type EventAttendee,
  type EventRecord,
  type InvitationEmailDelivery,
} from '../api/client';
import { useAuth } from '../auth/useAuth';

type EventDetailState =
  | {
      status: 'loading';
      event: null;
      attachments: EventAttachmentRecord[];
      attendees: EventAttendee[];
      dashboardEvent: null;
      error: null;
    }
  | {
      status: 'ready';
      event: EventRecord;
      attachments: EventAttachmentRecord[];
      attendees: EventAttendee[];
      dashboardEvent: DashboardEventSummary | null;
      error: null;
    }
  | {
      status: 'error';
      event: null;
      attachments: EventAttachmentRecord[];
      attendees: EventAttendee[];
      dashboardEvent: null;
      error: string;
    };

type InviteFeedback =
  | { tone: 'success' | 'error'; message: string; deliveries: InvitationEmailDelivery[] }
  | null;

export function EventDetailPage() {
  const { eventId } = useParams();
  const auth = useAuth();
  const displayName =
    auth.user?.name?.trim() || auth.user?.email.split('@')[0] || 'You';
  const [detail, setDetail] = useState<EventDetailState>({
    status: 'loading',
    event: null,
    attachments: [],
    attendees: [],
    dashboardEvent: null,
    error: null,
  });
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [inviteEmails, setInviteEmails] = useState('');
  const [inviteMessage, setInviteMessage] = useState('');
  const [inviteSending, setInviteSending] = useState(false);
  const [inviteFeedback, setInviteFeedback] = useState<InviteFeedback>(null);

  useEffect(() => {
    let cancelled = false;
    const currentUserSub = auth.user?.sub;

    const currentEventId = eventId ?? '';

    if (!currentEventId) {
      return () => {
        cancelled = true;
      };
    }

    async function loadEventDetail() {
      try {
        const [event, attachmentResponse, dashboardResponse] =
          await Promise.all([
            apiClient.event(currentEventId),
            apiClient.eventAttachments(currentEventId),
            apiClient.dashboardEvents(100).catch(() => ({ events: [] })),
          ]);

        const attendees =
          currentUserSub && event.owner_sub === currentUserSub
            ? await apiClient
                .eventAttendees(event.id)
                .then((response) => response.attendees)
                .catch(() => [])
            : [];

        if (cancelled) {
          return;
        }

        setDetail({
          status: 'ready',
          event,
          attachments: attachmentResponse.attachments,
          attendees,
          dashboardEvent:
            dashboardResponse.events.find((item) => item.id === event.id) ??
            null,
          error: null,
        });
      } catch (error) {
        if (cancelled) {
          return;
        }

        setDetail({
          status: 'error',
          event: null,
          attachments: [],
          attendees: [],
          dashboardEvent: null,
          error:
            error instanceof Error
              ? error.message
              : 'We could not load this event.',
        });
      }
    }

    void loadEventDetail();

    return () => {
      cancelled = true;
    };
  }, [eventId, auth.user?.sub]);

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
  const isOrganizer = detail.event.owner_sub === auth.user?.sub;
  const attendeeCounts = countAttendeeStatuses(detail.attendees);

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

  async function sendInvites() {
    if (detail.status !== 'ready') {
      return;
    }

    const currentEvent = detail.event;
    const currentAttendees = detail.attendees;
    const emails = parseInviteEmails(inviteEmails);

    if (!emails.length) {
      setInviteFeedback({
        tone: 'error',
        message: 'Add at least one email address.',
        deliveries: [],
      });
      return;
    }

    setInviteSending(true);
    setInviteFeedback(null);

    try {
      const response = await apiClient.sendEventInvitations(
        currentEvent.id,
        emails.map((email) => ({ email })),
        inviteMessage,
      );
      const attendeeResponse = await apiClient
        .eventAttendees(currentEvent.id)
        .catch(() => ({ attendees: currentAttendees }));
      const deliveries = response.invitations.map(
        (invitation) => invitation.email_delivery,
      );
      const sentCount = deliveries.filter(
        (delivery) => delivery.status === 'sent',
      ).length;

      setDetail((current) =>
        current.status === 'ready'
          ? {
              ...current,
              attendees: attendeeResponse.attendees,
            }
          : current,
      );
      setInviteEmails('');
      setInviteMessage('');
      setInviteFeedback({
        tone: 'success',
        message:
          sentCount === deliveries.length
            ? `Sent ${sentCount} invite${sentCount === 1 ? '' : 's'}.`
            : `Created ${deliveries.length} invite${deliveries.length === 1 ? '' : 's'} with ${sentCount} sent.`,
        deliveries,
      });
    } catch (error) {
      setInviteFeedback({
        tone: 'error',
        message:
          error instanceof Error
            ? error.message
            : 'Those invites could not be sent.',
        deliveries: [],
      });
    } finally {
      setInviteSending(false);
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

          {isOrganizer ? (
            <OrganizerInvitePanel
              attendees={detail.attendees}
              counts={attendeeCounts}
              feedback={inviteFeedback}
              inviteEmails={inviteEmails}
              inviteMessage={inviteMessage}
              isSending={inviteSending}
              onEmailsChange={setInviteEmails}
              onMessageChange={setInviteMessage}
              onSend={() => void sendInvites()}
            />
          ) : (
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
          )}
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

function OrganizerInvitePanel({
  attendees,
  counts,
  feedback,
  inviteEmails,
  inviteMessage,
  isSending,
  onEmailsChange,
  onMessageChange,
  onSend,
}: {
  attendees: EventAttendee[];
  counts: Record<string, number>;
  feedback: InviteFeedback;
  inviteEmails: string;
  inviteMessage: string;
  isSending: boolean;
  onEmailsChange: (value: string) => void;
  onMessageChange: (value: string) => void;
  onSend: () => void;
}) {
  return (
    <section className="rounded-lg border-4 border-ink bg-white p-5 shadow-sticker">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-black uppercase text-teal">Invites</p>
          <h3 className="mt-1 text-2xl font-black">Guest list</h3>
        </div>
        <div className="rounded-lg border-2 border-ink bg-sunny px-3 py-1 text-sm font-black shadow-sticker">
          {counts.accepted ?? 0} going
        </div>
      </div>

      <div className="mt-4 grid grid-cols-3 gap-2 text-center">
        {[
          ['Invited', counts.invited ?? 0, 'bg-paper'],
          ['Going', counts.accepted ?? 0, 'bg-mint'],
          ['Declined', counts.declined ?? 0, 'bg-coral text-white'],
        ].map(([label, value, className]) => (
          <div
            className={`rounded-lg border-2 border-ink px-2 py-2 ${className}`}
            key={label}
          >
            <p className="text-lg font-black">{value}</p>
            <p className="text-[0.65rem] font-black uppercase">{label}</p>
          </div>
        ))}
      </div>

      <form
        className="mt-5 space-y-3"
        onSubmit={(event) => {
          event.preventDefault();
          onSend();
        }}
      >
        <label className="block">
          <span className="text-sm font-black uppercase text-slate-600">
            Emails
          </span>
          <textarea
            className="mt-2 min-h-24 w-full rounded-lg border-2 border-ink bg-paper px-3 py-2 font-bold outline-none focus:bg-white"
            onChange={(event) => onEmailsChange(event.target.value)}
            placeholder="friend@example.com, crew@example.com"
            value={inviteEmails}
          />
        </label>
        <label className="block">
          <span className="text-sm font-black uppercase text-slate-600">
            Note
          </span>
          <textarea
            className="mt-2 min-h-20 w-full rounded-lg border-2 border-ink bg-paper px-3 py-2 outline-none focus:bg-white"
            maxLength={1000}
            onChange={(event) => onMessageChange(event.target.value)}
            placeholder="Bring your favorite snack."
            value={inviteMessage}
          />
        </label>
        <button
          className="flex min-h-11 w-full items-center justify-center rounded-lg border-2 border-ink bg-teal px-4 py-2 font-black text-white shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
          disabled={isSending}
          type="submit"
        >
          {isSending ? 'Sending' : 'Send invites'}
        </button>
      </form>

      {feedback ? (
        <div
          className={`mt-4 rounded-lg border-2 border-ink p-3 ${
            feedback.tone === 'success' ? 'bg-mint' : 'bg-coral text-white'
          }`}
        >
          <p className="font-black">{feedback.message}</p>
          {feedback.deliveries.length > 0 ? (
            <ul className="mt-2 space-y-1 text-sm font-bold">
              {feedback.deliveries.map((delivery) => (
                <li
                  className="flex items-center justify-between gap-2"
                  key={delivery.email}
                >
                  <span className="min-w-0 truncate">{delivery.email}</span>
                  <span className="shrink-0 uppercase">
                    {deliveryLabel(delivery.status)}
                  </span>
                </li>
              ))}
            </ul>
          ) : null}
        </div>
      ) : null}

      <ul className="mt-5 max-h-80 space-y-3 overflow-y-auto pr-1">
        {attendees.length > 0 ? (
          attendees.map((attendee) => (
            <li
              className="flex items-center justify-between gap-3 rounded-lg border-2 border-ink bg-paper px-3 py-2"
              key={attendee.invitation_id}
            >
              <div className="min-w-0">
                <p className="truncate font-black">{attendeeName(attendee)}</p>
                <p className="truncate text-sm font-bold text-slate-600">
                  {attendee.invitee_email ?? attendee.invitee_sub}
                </p>
              </div>
              <span
                className={`shrink-0 rounded-lg border-2 border-ink px-2 py-1 text-xs font-black uppercase ${statusBadgeClass(
                  attendee.invitation_status,
                )}`}
              >
                {attendeeStatusLabel(attendee)}
              </span>
            </li>
          ))
        ) : (
          <li className="rounded-lg border-2 border-ink bg-paper p-3">
            <p className="font-black">No guests yet</p>
            <p className="mt-1 text-sm text-slate-700">
              Send the first invite and the list will fill in here.
            </p>
          </li>
        )}
      </ul>
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

function parseInviteEmails(value: string) {
  const seen = new Set<string>();

  return value
    .split(/[\s,;]+/)
    .map((email) => email.trim().toLowerCase())
    .filter((email) => email.includes('@') && !email.startsWith('@') && !email.endsWith('@'))
    .filter((email) => {
      if (seen.has(email)) {
        return false;
      }

      seen.add(email);
      return true;
    });
}

function countAttendeeStatuses(attendees: EventAttendee[]) {
  return attendees.reduce<Record<string, number>>((counts, attendee) => {
    counts[attendee.invitation_status] =
      (counts[attendee.invitation_status] ?? 0) + 1;
    return counts;
  }, {});
}

function attendeeName(attendee: EventAttendee) {
  return (
    attendee.display_name?.trim() ||
    attendee.invitee_email?.split('@')[0] ||
    attendee.invitee_sub ||
    'Guest'
  );
}

function attendeeStatusLabel(attendee: EventAttendee) {
  if (attendee.rsvp_response === 'yes') {
    return 'Going';
  }

  if (attendee.rsvp_response === 'maybe') {
    return 'Maybe';
  }

  if (attendee.rsvp_response === 'no') {
    return 'No';
  }

  return relationshipLabel(attendee.invitation_status);
}

function deliveryLabel(status: InvitationEmailDelivery['status']) {
  switch (status) {
    case 'sent':
      return 'sent';
    case 'skipped':
      return 'queued';
    case 'rate_limited':
      return 'wait';
    case 'failed':
      return 'failed';
    default:
      return status;
  }
}

function statusBadgeClass(status: string) {
  switch (status) {
    case 'accepted':
      return 'bg-mint';
    case 'declined':
      return 'bg-coral text-white';
    case 'cancelled':
      return 'bg-slate-200';
    default:
      return 'bg-white';
  }
}
