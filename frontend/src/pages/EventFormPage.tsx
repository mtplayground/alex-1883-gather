import { useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';

import {
  apiClient,
  type EventAttachmentRecord,
  type EventDraftRequest,
  type EventRecord,
} from '../api/client';

type FormValues = {
  title: string;
  description: string;
  startsAt: string;
  timezone: string;
};

type LoadState = 'loading' | 'ready' | 'error';

const defaultTimezone =
  Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';

export function EventFormPage() {
  const { eventId } = useParams();
  const isEditing = Boolean(eventId);
  const [managedEventId, setManagedEventId] = useState(eventId ?? null);
  const [event, setEvent] = useState<EventRecord | null>(null);
  const [attachments, setAttachments] = useState<EventAttachmentRecord[]>([]);
  const [values, setValues] = useState<FormValues>(() => emptyFormValues());
  const [coverFile, setCoverFile] = useState<File | null>(null);
  const [attachmentFiles, setAttachmentFiles] = useState<File[]>([]);
  const [fileInputKey, setFileInputKey] = useState(0);
  const [loadState, setLoadState] = useState<LoadState>(
    isEditing ? 'loading' : 'ready',
  );
  const [saving, setSaving] = useState(false);
  const [deletingAttachmentId, setDeletingAttachmentId] = useState<string | null>(
    null,
  );
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    if (!eventId) {
      return () => {
        cancelled = true;
      };
    }

    Promise.all([apiClient.event(eventId), apiClient.eventAttachments(eventId)])
      .then(([loadedEvent, attachmentResponse]) => {
        if (cancelled) {
          return;
        }

        setManagedEventId(loadedEvent.id);
        setEvent(loadedEvent);
        setAttachments(attachmentResponse.attachments);
        setValues(formValuesFromEvent(loadedEvent));
        setLoadState('ready');
      })
      .catch((loadError: unknown) => {
        if (cancelled) {
          return;
        }

        setLoadState('error');
        setError(
          loadError instanceof Error
            ? loadError.message
            : 'We could not load this event.',
        );
      });

    return () => {
      cancelled = true;
    };
  }, [eventId]);

  const pendingAttachmentNames = useMemo(
    () => attachmentFiles.map((file) => file.name).join(', '),
    [attachmentFiles],
  );

  async function saveEvent(formEvent: React.FormEvent<HTMLFormElement>) {
    formEvent.preventDefault();
    setSaving(true);
    setError(null);
    setMessage(null);

    try {
      const draft = buildEventDraft(values, event?.cover_image_object_key);
      let savedEvent = managedEventId
        ? await apiClient.updateEvent(managedEventId, draft)
        : await apiClient.createEvent(draft);

      setManagedEventId(savedEvent.id);

      if (coverFile) {
        const coverResponse = await apiClient.uploadEventCoverImage(
          savedEvent.id,
          coverFile,
        );
        savedEvent = coverResponse.event;
      }

      const uploadedAttachments: EventAttachmentRecord[] = [];
      for (const file of attachmentFiles) {
        const uploadResponse = await apiClient.uploadEventAttachment(
          savedEvent.id,
          file,
        );
        uploadedAttachments.push(uploadResponse.attachment);
      }

      setEvent(savedEvent);
      setAttachments((current) => [...current, ...uploadedAttachments]);
      setCoverFile(null);
      setAttachmentFiles([]);
      setFileInputKey((key) => key + 1);
      setMessage('Saved. Cue the confetti.');
    } catch (saveError) {
      setError(
        saveError instanceof Error
          ? saveError.message
          : 'We could not save this event.',
      );
    } finally {
      setSaving(false);
    }
  }

  async function removeAttachment(attachment: EventAttachmentRecord) {
    const currentEventId = managedEventId;
    if (!currentEventId) {
      return;
    }

    setDeletingAttachmentId(attachment.id);
    setError(null);

    try {
      await apiClient.deleteEventAttachment(currentEventId, attachment.id);
      setAttachments((current) =>
        current.filter((item) => item.id !== attachment.id),
      );
      setMessage('Attachment removed.');
    } catch (deleteError) {
      setError(
        deleteError instanceof Error
          ? deleteError.message
          : 'We could not remove that attachment.',
      );
    } finally {
      setDeletingAttachmentId(null);
    }
  }

  if (loadState === 'loading') {
    return <EventFormLoading />;
  }

  if (loadState === 'error') {
    return (
      <section className="rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
        <p className="text-sm font-black uppercase text-coral">Event editor</p>
        <h2 className="mt-2 text-3xl font-black">This event did not load.</h2>
        <p className="mt-3 text-slate-700">{error}</p>
        <Link
          className="mt-5 inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
          to="/dashboard"
        >
          Back to dashboard
        </Link>
      </section>
    );
  }

  return (
    <section className="space-y-6">
      <div className="flex flex-col gap-4 rounded-lg border-4 border-ink bg-white p-6 shadow-sticker md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-sm font-black uppercase text-teal">
            {managedEventId ? 'Edit event' : 'Create event'}
          </p>
          <h2 className="mt-2 text-4xl font-black leading-tight">
            {managedEventId ? values.title || 'Untitled event' : 'Plan a gathering'}
          </h2>
        </div>
        <div className="flex flex-wrap gap-3">
          <Link
            className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-paper px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
            to="/dashboard"
          >
            Dashboard
          </Link>
          {managedEventId ? (
            <Link
              className="inline-flex min-h-11 items-center justify-center rounded-lg border-2 border-ink bg-sunny px-4 py-2 font-black shadow-sticker transition hover:-translate-y-0.5"
              to={`/events/${managedEventId}`}
            >
              View event
            </Link>
          ) : null}
        </div>
      </div>

      {message ? (
        <div className="rounded-lg border-4 border-ink bg-mint p-4 font-black shadow-sticker">
          {message}
        </div>
      ) : null}
      {error ? (
        <div className="rounded-lg border-4 border-ink bg-white p-4 font-bold text-coral shadow-sticker">
          {error}
        </div>
      ) : null}

      <form className="grid gap-6 xl:grid-cols-[1fr_0.75fr]" onSubmit={saveEvent}>
        <div className="space-y-5 rounded-lg border-4 border-ink bg-white p-6 shadow-sticker">
          <label className="block">
            <span className="text-sm font-black uppercase text-coral">Title</span>
            <input
              className="mt-2 min-h-12 w-full rounded-lg border-2 border-ink bg-paper px-4 py-2 text-lg font-black"
              maxLength={160}
              onChange={(input) =>
                setValues((current) => ({
                  ...current,
                  title: input.target.value,
                }))
              }
              required
              value={values.title}
            />
          </label>

          <label className="block">
            <span className="text-sm font-black uppercase text-coral">
              Description
            </span>
            <textarea
              className="mt-2 min-h-40 w-full resize-y rounded-lg border-2 border-ink bg-paper px-4 py-3 text-base"
              maxLength={5000}
              onChange={(input) =>
                setValues((current) => ({
                  ...current,
                  description: input.target.value,
                }))
              }
              value={values.description}
            />
          </label>

          <div className="grid gap-4 md:grid-cols-2">
            <label className="block">
              <span className="text-sm font-black uppercase text-coral">
                Date and time
              </span>
              <input
                className="mt-2 min-h-12 w-full rounded-lg border-2 border-ink bg-paper px-4 py-2 font-bold"
                onChange={(input) =>
                  setValues((current) => ({
                    ...current,
                    startsAt: input.target.value,
                  }))
                }
                required
                type="datetime-local"
                value={values.startsAt}
              />
            </label>

            <label className="block">
              <span className="text-sm font-black uppercase text-coral">
                Time zone
              </span>
              <input
                className="mt-2 min-h-12 w-full rounded-lg border-2 border-ink bg-paper px-4 py-2 font-bold"
                maxLength={80}
                onChange={(input) =>
                  setValues((current) => ({
                    ...current,
                    timezone: input.target.value,
                  }))
                }
                value={values.timezone}
              />
            </label>
          </div>

          <button
            className="inline-flex min-h-12 items-center justify-center rounded-lg border-2 border-ink bg-teal px-5 py-2 font-black text-white shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
            disabled={saving}
            type="submit"
          >
            {saving ? 'Saving' : managedEventId ? 'Save event' : 'Create event'}
          </button>
        </div>

        <aside className="space-y-5">
          <section className="rounded-lg border-4 border-ink bg-paper p-5 shadow-sticker">
            <p className="text-sm font-black uppercase text-teal">Cover image</p>
            <div className="mt-4 aspect-[4/3] overflow-hidden rounded-lg border-2 border-ink bg-mint">
              {event?.cover_image_object_key ? (
                <div className="flex h-full items-center justify-center p-4 text-center font-black">
                  Cover stored
                </div>
              ) : (
                <div className="flex h-full items-center justify-center text-6xl">
                  🎪
                </div>
              )}
            </div>
            <label className="mt-4 block">
              <span className="text-sm font-black uppercase text-coral">
                Replace cover
              </span>
              <input
                accept="image/jpeg,image/png,image/webp,image/gif"
                className="mt-2 block w-full rounded-lg border-2 border-ink bg-white p-3 font-bold"
                key={`cover-${fileInputKey}`}
                onChange={(input) =>
                  setCoverFile(input.target.files?.item(0) ?? null)
                }
                type="file"
              />
            </label>
            {coverFile ? (
              <p className="mt-2 text-sm font-bold text-slate-700">
                {coverFile.name}
              </p>
            ) : null}
          </section>

          <section className="rounded-lg border-4 border-ink bg-white p-5 shadow-sticker">
            <p className="text-sm font-black uppercase text-teal">PDFs</p>
            <label className="mt-4 block">
              <span className="text-sm font-black uppercase text-coral">
                Add attachments
              </span>
              <input
                accept="application/pdf"
                className="mt-2 block w-full rounded-lg border-2 border-ink bg-paper p-3 font-bold"
                key={`attachments-${fileInputKey}`}
                multiple
                onChange={(input) =>
                  setAttachmentFiles(Array.from(input.target.files ?? []))
                }
                type="file"
              />
            </label>
            {pendingAttachmentNames ? (
              <p className="mt-2 text-sm font-bold text-slate-700">
                {pendingAttachmentNames}
              </p>
            ) : null}

            <div className="mt-5 space-y-3">
              {attachments.length === 0 ? (
                <div className="rounded-lg border-2 border-ink bg-paper p-4">
                  <p className="font-black">No PDFs yet</p>
                </div>
              ) : (
                attachments.map((attachment) => (
                  <div
                    className="flex flex-col gap-3 rounded-lg border-2 border-ink bg-paper p-3 sm:flex-row sm:items-center sm:justify-between"
                    key={attachment.id}
                  >
                    <div className="min-w-0">
                      <p className="truncate font-black">
                        {attachment.filename}
                      </p>
                      <p className="text-sm text-slate-600">
                        {formatBytes(attachment.byte_size)}
                      </p>
                    </div>
                    <button
                      className="inline-flex min-h-10 items-center justify-center rounded-lg border-2 border-ink bg-white px-3 py-1 font-black shadow-sticker transition hover:-translate-y-0.5 disabled:cursor-wait disabled:opacity-70"
                      disabled={deletingAttachmentId === attachment.id}
                      onClick={() => void removeAttachment(attachment)}
                      type="button"
                    >
                      {deletingAttachmentId === attachment.id
                        ? 'Removing'
                        : 'Remove'}
                    </button>
                  </div>
                ))
              )}
            </div>
          </section>
        </aside>
      </form>
    </section>
  );
}

function EventFormLoading() {
  return (
    <section className="grid gap-6 xl:grid-cols-[1fr_0.75fr]">
      <div className="h-[34rem] animate-pulse rounded-lg border-4 border-ink bg-white shadow-sticker" />
      <div className="h-[30rem] animate-pulse rounded-lg border-4 border-ink bg-paper shadow-sticker" />
    </section>
  );
}

function emptyFormValues(): FormValues {
  return {
    title: '',
    description: '',
    startsAt: toLocalInputValue(new Date(Date.now() + 60 * 60 * 1000)),
    timezone: defaultTimezone,
  };
}

function formValuesFromEvent(event: EventRecord): FormValues {
  return {
    title: event.title,
    description: event.description ?? '',
    startsAt: toLocalInputValue(new Date(event.starts_at)),
    timezone: event.timezone ?? defaultTimezone,
  };
}

function buildEventDraft(
  values: FormValues,
  coverImageObjectKey: string | null | undefined,
): EventDraftRequest {
  return {
    title: values.title.trim(),
    description: values.description.trim() || null,
    starts_at: new Date(values.startsAt).toISOString(),
    timezone: values.timezone.trim() || null,
    cover_image_object_key: coverImageObjectKey ?? null,
  };
}

function toLocalInputValue(date: Date) {
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());

  return `${year}-${month}-${day}T${hours}:${minutes}`;
}

function pad(value: number) {
  return String(value).padStart(2, '0');
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
