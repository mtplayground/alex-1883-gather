import { ApiError } from './client';

export type ErrorContext =
  | 'auth'
  | 'comment'
  | 'dashboard'
  | 'download'
  | 'eventLoad'
  | 'eventSave'
  | 'invite'
  | 'profile'
  | 'rsvp'
  | 'upload';

const contextFallbacks: Record<ErrorContext, string> = {
  auth: 'That account step did not land. Give it another try.',
  comment: 'That comment did not post. Try again in a moment.',
  dashboard: 'Your event list did not load. Refresh and we will try again.',
  download: 'That download is not ready. Try again in a moment.',
  eventLoad:
    'That event did not load. It may be private or no longer available.',
  eventSave: 'That event did not save. Check the details and try again.',
  invite: 'Those invites did not go out. Check the emails and try again.',
  profile: 'That profile update did not save. Try again in a moment.',
  rsvp: 'That RSVP did not save. Try again in a moment.',
  upload: 'That upload did not land. Choose the file again and retry.',
};

const codeMessages: Record<string, string> = {
  attachment_forbidden:
    'Only the organizer or uploader can change that attachment.',
  empty_upload: 'That file looks empty. Choose another one.',
  event_forbidden: 'That event is private to its organizer and guest list.',
  invalid_pdf: 'That file does not look like a valid PDF.',
  invalid_upload:
    'That upload did not come through cleanly. Choose the file again.',
  invitation_forbidden: 'That invite belongs to another guest.',
  missing_cover_image: 'Add a cover image before uploading.',
  missing_photo: 'Add a profile photo before uploading.',
  not_authenticated:
    'Your session has expired. Sign in again and we will bring you back.',
  unsupported_attachment_type: 'Attachments need to be PDF files.',
  unsupported_image_type: 'Pick a JPEG, PNG, WebP, or GIF image.',
  upload_too_large: 'That file is too large. Pick a smaller one and try again.',
  validation_failed: 'A few details need another look.',
};

export function friendlyErrorMessage(
  error: unknown,
  context: ErrorContext,
): string {
  if (error instanceof ApiError) {
    if (error.code && codeMessages[error.code]) {
      return codeMessages[error.code];
    }

    if (error.status === 401) {
      return 'Your session has expired. Sign in again and we will bring you back.';
    }

    if (error.status === 403) {
      return context === 'invite'
        ? 'Only the organizer can send invites for that event.'
        : 'That page is private to the right guest list.';
    }

    if (error.status === 404) {
      return 'We could not find that. It may have been moved or deleted.';
    }

    if (error.status === 413) {
      return 'That file is too large. Pick a smaller one and try again.';
    }

    if (error.status === 422) {
      return 'A few details need another look.';
    }

    if (error.status >= 500) {
      return 'Something tripped on our side. Try again in a moment.';
    }

    return error.message || contextFallbacks[context];
  }

  if (error instanceof TypeError) {
    return 'We could not reach the server. Check your connection and try again.';
  }

  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }

  return contextFallbacks[context];
}
