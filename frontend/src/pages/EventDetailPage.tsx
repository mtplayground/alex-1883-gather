import { useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import {
  apiClient,
  type DashboardEventSummary,
  type EventAttachmentRecord,
  type EventAttendee,
  type EventCommentRecord,
  type EventRsvpUpdateRequest,
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
      comments: EventCommentRecord[];
      dashboardEvent: null;
      error: null;
    }
  | {
      status: 'ready';
      event: EventRecord;
      attachments: EventAttachmentRecord[];
      attendees: EventAttendee[];
      comments: EventCommentRecord[];
      dashboardEvent: DashboardEventSummary | null;
      error: null;
    }
  | {
      status: 'error';
      event: null;
      attachments: EventAttachmentRecord[];
      attendees: EventAttendee[];
      comments: EventCommentRecord[];
      dashboardEvent: null;
      error: string;
    };

type InviteFeedback =
  | { tone: 'success' | 'error'; message: string; deliveries: InvitationEmailDelivery[] }
  | null;

type RsvpFeedback = { tone: 'success' | 'error'; message: string } | null;

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
    comments: [],
    dashboardEvent: null,
    error: null,
  });
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [downloadError, setDownloadError] = useState<string | null>(null);
  const [inviteEmails, setInviteEmails] = useState('');
  const [inviteMessage, setInviteMessage] = useState('');
  const [inviteSending, setInviteSending] = useState(false);
  const [inviteFeedback, setInviteFeedback] = useState<InviteFeedback>(null);
  const [rsvpChoice, setRsvpChoice] =
    useState<EventRsvpUpdateRequest['response']>('maybe');
  const [rsvpNote, setRsvpNote] = useState('');
  const [rsvpSaving, setRsvpSaving] = useState(false);
  const [rsvpFeedback, setRsvpFeedback] = useState<RsvpFeedback>(null);
  const [rsvpCelebrating, setRsvpCelebrating] = useState(false);
  const [commentBody, setCommentBody] = useState('');
  const [commentSaving, setCommentSaving] = useState(false);
  const [commentError, setCommentError] = useState<string | null>(null);
  const [celebratedCommentId, setCelebratedCommentId] = useState<string | null>(
    null,
  );

  useEffect(() => {
    let cancelled = false;
    const currentUser = auth.user;

    const currentEventId = eventId ?? '';

    if (!currentEventId) {
      return () => {
        cancelled = true;
      };
    }

    async function loadEventDetail() {
      try {
        const [event, attachmentResponse, dashboardResponse, commentResponse] =
          await Promise.all([
            apiClient.event(currentEventId),
            apiClient.eventAttachments(currentEventId),
            apiClient.dashboardEvents(100).catch(() => ({ events: [] })),
            apiClient
              .eventComments(currentEventId)
              .catch(() => ({ comments: [] })),
          ]);

        const attendees = await apiClient
          .eventAttendees(event.id)
          .then((response) => response.attendees)
          .catch(() => []);

        if (cancelled) {
          return;
        }

        const attendee =
          currentUser === null
            ? null
            : findCurrentAttendee(attendees, currentUser.sub, currentUser.email);

        if (attendee) {
          setRsvpChoice(toRsvpChoice(attendee));
          setRsvpNote(attendee.rsvp_note ?? '');
        }

        setDetail({
          status: 'ready',
          event,
          attachments: attachmentResponse.attachments,
          attendees,
          comments: commentResponse.comments,
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
          comments: [],
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
  }, [eventId, auth.user]);

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
  const socialAttendees = socialAttendeeList(detail.attendees);

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

  async function saveRsvp() {
    if (detail.status !== 'ready') {
      return;
    }

    const currentEvent = detail.event;
    const currentAttendees = detail.attendees;

    setRsvpSaving(true);
    setRsvpFeedback(null);

    try {
      await apiClient.updateEventRsvp(currentEvent.id, {
        response: rsvpChoice,
        note: rsvpNote.trim() ? rsvpNote.trim() : null,
      });
      const attendeeResponse = await apiClient
        .eventAttendees(currentEvent.id)
        .catch(() => ({ attendees: currentAttendees }));

      setDetail((current) =>
        current.status === 'ready'
          ? {
              ...current,
              attendees: attendeeResponse.attendees,
              dashboardEvent: current.dashboardEvent
                ? {
                    ...current.dashboardEvent,
                    relationship: rsvpChoice === 'no' ? 'invited' : 'joined',
                  }
                : current.dashboardEvent,
            }
          : current,
      );
      setRsvpFeedback({
        tone: 'success',
        message: rsvpChoice === 'yes' ? "You're on the list." : 'RSVP saved.',
      });

      if (rsvpChoice === 'yes') {
        setRsvpCelebrating(true);
        window.setTimeout(() => setRsvpCelebrating(false), 1400);
      }
    } catch (error) {
      setRsvpFeedback({
        tone: 'error',
        message:
          error instanceof Error ? error.message : 'We could not save that RSVP.',
      });
    } finally {
      setRsvpSaving(false);
    }
  }

  async function postComment() {
    if (detail.status !== 'ready') {
      return;
    }

    const body = commentBody.trim();

    if (!body) {
      setCommentError('Write a quick note first.');
      return;
    }

    setCommentSaving(true);
    setCommentError(null);

    try {
      const response = await apiClient.createEventComment(detail.event.id, body);
      setDetail((current) =>
        current.status === 'ready'
          ? {
              ...current,
              comments: [...current.comments, response.comment],
            }
          : current,
      );
      setCommentBody('');
      setCelebratedCommentId(response.comment.id);
      window.setTimeout(() => setCelebratedCommentId(null), 1400);
    } catch (error) {
      setCommentError(
        error instanceof Error
          ? error.message
          : 'We could not post that comment.',
      );
    } finally {
      setCommentSaving(false);
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
          {isOrganizer ? (
            <Link
              className="flex min-h-12 items-center justify-center rounded-lg border-4 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
              to={`/events/${detail.event.id}/edit`}
            >
              Edit event
            </Link>
          ) : null}
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
            <MemberRsvpPanel
              choice={rsvpChoice}
              displayName={displayName}
              feedback={rsvpFeedback}
              isCelebrating={rsvpCelebrating}
              isSaving={rsvpSaving}
              note={rsvpNote}
              onChoiceChange={setRsvpChoice}
              onNoteChange={setRsvpNote}
              onSave={() => void saveRsvp()}
              relationship={relationship}
            />
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

      <div className="grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
        <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
          <p className="text-sm font-black uppercase text-teal">Activity</p>
          <div className="mt-4 grid gap-3 md:grid-cols-3 xl:grid-cols-1">
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

        <CommentThread
          celebratedCommentId={celebratedCommentId}
          comments={detail.comments}
          currentUserSub={auth.user?.sub ?? ''}
          error={commentError}
          isSaving={commentSaving}
          onBodyChange={setCommentBody}
          onSubmit={() => void postComment()}
          value={commentBody}
        />
      </div>

      <SocialAttendeeList attendees={socialAttendees} counts={attendeeCounts} />
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

function MemberRsvpPanel({
  choice,
  displayName,
  feedback,
  isCelebrating,
  isSaving,
  note,
  onChoiceChange,
  onNoteChange,
  onSave,
  relationship,
}: {
  choice: EventRsvpUpdateRequest['response'];
  displayName: string;
  feedback: RsvpFeedback;
  isCelebrating: boolean;
  isSaving: boolean;
  note: string;
  onChoiceChange: (value: EventRsvpUpdateRequest['response']) => void;
  onNoteChange: (value: string) => void;
  onSave: () => void;
  relationship: string;
}) {
  return (
    <section className="relative overflow-hidden rounded-lg border-4 border-ink bg-white p-5 shadow-sticker">
      {isCelebrating ? (
        <div className="pointer-events-none absolute right-4 top-4 animate-bounce rounded-lg border-2 border-ink bg-sunny px-3 py-1 text-sm font-black shadow-sticker">
          You&apos;re in!
        </div>
      ) : null}

      <p className="text-sm font-black uppercase text-teal">Your RSVP</p>
      <h3 className="mt-1 text-2xl font-black">{displayName}</h3>
      <p className="mt-1 text-sm font-bold text-slate-600">{relationship}</p>

      <div className="mt-4 grid grid-cols-3 gap-2">
        {[
          ['yes', 'Going', 'bg-mint'],
          ['maybe', 'Maybe', 'bg-sunny'],
          ['no', 'No', 'bg-coral text-white'],
        ].map(([value, label, activeClass]) => {
          const active = choice === value;

          return (
            <button
              className={`min-h-12 rounded-lg border-2 border-ink px-2 py-2 text-sm font-black shadow-sticker transition hover:-translate-y-0.5 ${
                active ? activeClass : 'bg-paper'
              }`}
              key={value}
              onClick={() =>
                onChoiceChange(value as EventRsvpUpdateRequest['response'])
              }
              type="button"
            >
              {label}
            </button>
          );
        })}
      </div>

      <label className="mt-4 block">
        <span className="text-sm font-black uppercase text-slate-600">
          Note
        </span>
        <textarea
          className="mt-2 min-h-20 w-full rounded-lg border-2 border-ink bg-paper px-3 py-2 outline-none focus:bg-white"
          maxLength={1000}
          onChange={(event) => onNoteChange(event.target.value)}
          placeholder="See you there."
          value={note}
        />
      </label>

      <button
        className="mt-3 flex min-h-11 w-full items-center justify-center rounded-lg border-2 border-ink bg-teal px-4 py-2 font-black text-white shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
        disabled={isSaving}
        onClick={onSave}
        type="button"
      >
        {isSaving ? 'Saving' : 'Save RSVP'}
      </button>

      {feedback ? (
        <div
          className={`mt-4 rounded-lg border-2 border-ink p-3 font-black ${
            feedback.tone === 'success' ? 'bg-mint' : 'bg-coral text-white'
          }`}
        >
          {feedback.message}
        </div>
      ) : null}
    </section>
  );
}

function CommentThread({
  celebratedCommentId,
  comments,
  currentUserSub,
  error,
  isSaving,
  onBodyChange,
  onSubmit,
  value,
}: {
  celebratedCommentId: string | null;
  comments: EventCommentRecord[];
  currentUserSub: string;
  error: string | null;
  isSaving: boolean;
  onBodyChange: (value: string) => void;
  onSubmit: () => void;
  value: string;
}) {
  return (
    <section className="relative overflow-hidden rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker">
      {celebratedCommentId ? (
        <div className="pointer-events-none absolute right-4 top-4 animate-bounce rounded-lg border-2 border-ink bg-sunny px-3 py-1 text-sm font-black shadow-sticker">
          New comment
        </div>
      ) : null}

      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="text-sm font-black uppercase text-coral">Comments</p>
          <h3 className="mt-1 text-3xl font-black">Event thread</h3>
        </div>
        <div className="rounded-lg border-2 border-ink bg-white px-3 py-2 text-sm font-black shadow-sticker">
          {comments.length} note{comments.length === 1 ? '' : 's'}
        </div>
      </div>

      <div className="mt-5 max-h-[28rem] space-y-4 overflow-y-auto pr-1">
        {comments.length > 0 ? (
          comments.map((comment) => {
            const mine = comment.author.sub === currentUserSub;
            const isFresh = comment.id === celebratedCommentId;

            return (
              <div
                className={`flex gap-3 ${mine ? 'justify-end' : 'justify-start'} ${
                  isFresh ? 'animate-pulse' : ''
                }`}
                key={comment.id}
              >
                {!mine ? <CommentAvatar comment={comment} /> : null}
                <div
                  className={`max-w-[min(32rem,82%)] rounded-lg border-2 border-ink px-4 py-3 shadow-sticker ${
                    mine ? 'bg-teal text-white' : 'bg-white'
                  }`}
                >
                  <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-1">
                    <p className="font-black">{commentAuthorName(comment)}</p>
                    <p
                      className={`text-xs font-bold ${
                        mine ? 'text-white/80' : 'text-slate-500'
                      }`}
                    >
                      {formatCommentTime(comment.created_at)}
                    </p>
                  </div>
                  <p className="mt-2 whitespace-pre-line break-words leading-relaxed">
                    {comment.body}
                  </p>
                </div>
                {mine ? <CommentAvatar comment={comment} /> : null}
              </div>
            );
          })
        ) : (
          <div className="rounded-lg border-2 border-ink bg-white p-4">
            <p className="font-black">No comments yet</p>
            <p className="mt-1 text-slate-700">
              Start the thread with a quick update for everyone.
            </p>
          </div>
        )}
      </div>

      <form
        className="mt-5 space-y-3"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmit();
        }}
      >
        <label className="block">
          <span className="text-sm font-black uppercase text-slate-600">
            Add a comment
          </span>
          <textarea
            className="mt-2 min-h-24 w-full rounded-lg border-2 border-ink bg-white px-3 py-2 font-bold outline-none focus:bg-mint"
            maxLength={2000}
            onChange={(event) => onBodyChange(event.target.value)}
            placeholder="Drop a detail, question, or quick update."
            value={value}
          />
        </label>

        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
          {error ? (
            <p className="rounded-lg border-2 border-ink bg-coral px-3 py-2 text-sm font-black text-white">
              {error}
            </p>
          ) : (
            <p className="text-sm font-bold text-slate-600">
              {value.trim().length}/2000
            </p>
          )}
          <button
            className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-5 py-2 font-black shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
            disabled={isSaving}
            type="submit"
          >
            {isSaving ? 'Posting' : 'Post comment'}
          </button>
        </div>
      </form>
    </section>
  );
}

function CommentAvatar({ comment }: { comment: EventCommentRecord }) {
  return comment.author.picture_url ? (
    <img
      alt=""
      className="mt-1 size-11 shrink-0 rounded-lg border-2 border-ink object-cover shadow-sticker"
      src={comment.author.picture_url}
    />
  ) : (
    <div className="mt-1 flex size-11 shrink-0 items-center justify-center rounded-lg border-2 border-ink bg-mint font-black shadow-sticker">
      {commentAuthorName(comment).slice(0, 1).toUpperCase()}
    </div>
  );
}

function SocialAttendeeList({
  attendees,
  counts,
}: {
  attendees: EventAttendee[];
  counts: Record<string, number>;
}) {
  return (
    <section className="rounded-lg border-4 border-ink bg-paper p-6 shadow-sticker">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <p className="text-sm font-black uppercase text-coral">
            Who&apos;s coming
          </p>
          <h3 className="mt-1 text-3xl font-black">
            {(counts.accepted ?? 0) + (counts.invited ?? 0)} people in the mix
          </h3>
        </div>
        <div className="rounded-lg border-2 border-ink bg-white px-3 py-2 text-sm font-black shadow-sticker">
          {counts.accepted ?? 0} going
        </div>
      </div>

      {attendees.length > 0 ? (
        <ul className="mt-5 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {attendees.map((attendee) => (
            <li
              className="flex min-w-0 items-center gap-3 rounded-lg border-2 border-ink bg-white p-3"
              key={attendee.invitation_id}
            >
              {attendee.picture_url ? (
                <img
                  alt=""
                  className="size-12 shrink-0 rounded-lg border-2 border-ink object-cover"
                  src={attendee.picture_url}
                />
              ) : (
                <div className="flex size-12 shrink-0 items-center justify-center rounded-lg border-2 border-ink bg-mint text-lg font-black">
                  {attendeeName(attendee).slice(0, 1).toUpperCase()}
                </div>
              )}
              <div className="min-w-0">
                <p className="truncate font-black">{attendeeName(attendee)}</p>
                <p className="text-sm font-bold text-slate-600">
                  {attendeeStatusLabel(attendee)}
                </p>
              </div>
            </li>
          ))}
        </ul>
      ) : (
        <div className="mt-5 rounded-lg border-2 border-ink bg-white p-4">
          <p className="font-black">No confirmed guests yet</p>
          <p className="mt-1 text-slate-700">
            RSVP updates will turn this into a friendly roll call.
          </p>
        </div>
      )}
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

function formatCommentTime(value: string) {
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
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

function findCurrentAttendee(
  attendees: EventAttendee[],
  userSub: string,
  userEmail: string,
) {
  return (
    attendees.find((attendee) => attendee.invitee_sub === userSub) ??
    attendees.find((attendee) =>
      attendee.invitee_email?.toLowerCase() === userEmail.toLowerCase()
    ) ??
    null
  );
}

function toRsvpChoice(
  attendee: EventAttendee,
): EventRsvpUpdateRequest['response'] {
  if (
    attendee.rsvp_response === 'yes' ||
    attendee.rsvp_response === 'no' ||
    attendee.rsvp_response === 'maybe'
  ) {
    return attendee.rsvp_response;
  }

  if (attendee.invitation_status === 'accepted') {
    return 'yes';
  }

  if (attendee.invitation_status === 'declined') {
    return 'no';
  }

  return 'maybe';
}

function socialAttendeeList(attendees: EventAttendee[]) {
  return attendees.filter((attendee) => {
    if (attendee.rsvp_response === 'no') {
      return false;
    }

    if (attendee.rsvp_response === 'yes' || attendee.rsvp_response === 'maybe') {
      return true;
    }

    return attendee.invitation_status === 'accepted';
  });
}

function attendeeName(attendee: EventAttendee) {
  return (
    attendee.display_name?.trim() ||
    attendee.invitee_email?.split('@')[0] ||
    attendee.invitee_sub ||
    'Guest'
  );
}

function commentAuthorName(comment: EventCommentRecord) {
  return (
    comment.author.name?.trim() ||
    comment.author.email.split('@')[0] ||
    comment.author.sub ||
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
