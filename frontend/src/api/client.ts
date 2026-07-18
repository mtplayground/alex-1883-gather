import { config } from '../config';

export type HealthResponse = {
  status: string;
  service: string;
  database: string;
  email: string;
  object_storage: string;
};

export type CurrentUserResponse = {
  sub: string;
  email: string;
  email_verified: boolean;
  name: string | null;
  picture_url: string | null;
  registered: boolean;
};

export type EmailDelivery = {
  status: 'sent' | 'skipped' | 'rate_limited' | 'failed';
  id: string | null;
  message: string | null;
};

export type AuthActionResponse = {
  user: CurrentUserResponse;
  message: string;
  email_delivery: EmailDelivery;
};

export type VerificationStatusResponse = {
  verified: boolean;
  message: string;
};

export type PasswordResetResponse = {
  message: string;
  login_url: string;
  email_delivery: EmailDelivery;
};

export type AccountSettingsResponse = {
  sub: string;
  email: string;
  email_verified: boolean;
  name: string | null;
  picture_url: string | null;
  created_at: string;
  last_seen_at: string;
};

export type ProfileRecord = {
  user_sub: string;
  display_name: string;
  photo_object_key: string | null;
  bio: string | null;
  created_at: string;
  updated_at: string;
};

export type ProfileResponse = {
  account: AccountSettingsResponse;
  profile: ProfileRecord;
};

export type ProfileUpdateRequest = {
  display_name: string;
  photo_object_key?: string | null;
  bio?: string | null;
};

export type ProfilePhotoResponse = {
  profile: ProfileRecord;
  object_key: string;
  content_type: string;
  access_url: string;
};

export type DashboardEventSummary = {
  id: string;
  owner_sub: string;
  title: string;
  description: string | null;
  starts_at: string;
  timezone: string | null;
  cover_image_object_key: string | null;
  cover_image_url: string | null;
  relationship: 'organizer' | 'joined' | 'invited' | string;
  created_at: string;
  updated_at: string;
};

export type DashboardEventsResponse = {
  events: DashboardEventSummary[];
};

export type EventRecord = {
  id: string;
  owner_sub: string;
  title: string;
  description: string | null;
  starts_at: string;
  timezone: string | null;
  cover_image_object_key: string | null;
  created_at: string;
  updated_at: string;
};

export type EventDraftRequest = {
  title: string;
  description?: string | null;
  starts_at: string;
  timezone?: string | null;
  cover_image_object_key?: string | null;
};

export type EventAttachmentRecord = {
  id: string;
  event_id: string;
  uploaded_by_sub: string;
  object_key: string;
  filename: string;
  content_type: string;
  byte_size: number;
  page_count: number | null;
  metadata: unknown;
  created_at: string;
  updated_at: string;
};

export type EventAttachmentListResponse = {
  attachments: EventAttachmentRecord[];
};

export type EventAttachmentDownloadResponse = {
  attachment: EventAttachmentRecord;
  access_url: string;
};

export type EventCoverImageResponse = {
  event: EventRecord;
  object_key: string;
  content_type: string;
  access_url: string;
};

export type EventAttachmentUploadResponse = {
  attachment: EventAttachmentRecord;
  access_url: string;
};

export type EventInvitationRecord = {
  id: string;
  event_id: string;
  inviter_sub: string;
  invitee_sub: string | null;
  invitee_email: string | null;
  status: 'invited' | 'accepted' | 'declined' | 'cancelled' | string;
  message: string | null;
  created_at: string;
  updated_at: string;
};

export type InvitationEmailDelivery = {
  email: string;
  status: 'sent' | 'skipped' | 'rate_limited' | 'failed';
  id: string | null;
  message: string | null;
};

export type SentInvitation = {
  invitation: EventInvitationRecord;
  email_delivery: InvitationEmailDelivery;
};

export type SendInvitationsResponse = {
  invitations: SentInvitation[];
};

export type InvitationRecipientRequest = {
  email: string;
  name?: string | null;
};

export type EventAttendee = {
  invitation_id: string;
  event_id: string;
  invitee_sub: string | null;
  invitee_email: string | null;
  display_name: string | null;
  picture_url: string | null;
  invitation_status: string;
  rsvp_response: string | null;
  rsvp_note: string | null;
  responded_at: string | null;
  updated_at: string;
};

export type EventAttendeeListResponse = {
  attendees: EventAttendee[];
};

export type EventCommentAuthor = {
  sub: string;
  email: string;
  name: string | null;
  picture_url: string | null;
};

export type EventCommentRecord = {
  id: string;
  event_id: string;
  author: EventCommentAuthor;
  body: string;
  created_at: string;
  updated_at: string;
};

export type EventCommentListResponse = {
  comments: EventCommentRecord[];
};

export type EventCommentCreateResponse = {
  comment: EventCommentRecord;
};

export type EventActivityActor = {
  sub: string;
  email: string | null;
  name: string | null;
  picture_url: string | null;
};

export type EventActivityRecord = {
  id: string;
  event_id: string;
  actor: EventActivityActor | null;
  activity_type: string;
  message: string;
  payload: unknown;
  created_at: string;
};

export type EventActivityListResponse = {
  activity: EventActivityRecord[];
};

export type EventRsvpUpdateRequest = {
  response: 'yes' | 'no' | 'maybe';
  note?: string | null;
};

export type EventRsvpRecord = {
  id: string;
  invitation_id: string;
  event_id: string;
  user_sub: string;
  response: string;
  note: string | null;
  responded_at: string;
  created_at: string;
  updated_at: string;
};

export type RsvpActionResponse = {
  invitation: EventInvitationRecord;
  rsvp: EventRsvpRecord;
  email_delivery: InvitationEmailDelivery;
};

class ApiClient {
  constructor(private readonly baseUrl: string) {}

  health() {
    return this.get<HealthResponse>('/api/health');
  }

  me() {
    return this.get<CurrentUserResponse>('/api/me');
  }

  register() {
    return this.post<AuthActionResponse>('/api/auth/register');
  }

  verifyEmail() {
    return this.get<VerificationStatusResponse>('/api/auth/verify');
  }

  requestPasswordReset(email: string, returnTo = '/dashboard') {
    return this.post<PasswordResetResponse>(
      '/api/auth/password-reset/request',
      {
        email,
        return_to: this.frontendReturnTo(returnTo),
      },
    );
  }

  completePasswordReset(returnTo = '/dashboard') {
    return this.post<PasswordResetResponse>(
      '/api/auth/password-reset/complete',
      {
        return_to: this.frontendReturnTo(returnTo),
      },
    );
  }

  loginUrl(returnPath = '/dashboard') {
    return this.authRedirectUrl('/api/auth/login', returnPath);
  }

  googleLoginUrl(returnPath = '/dashboard') {
    return this.authRedirectUrl('/api/auth/google', returnPath);
  }

  profile() {
    return this.get<ProfileResponse>('/api/profile');
  }

  updateProfile(update: ProfileUpdateRequest) {
    return this.put<ProfileResponse>('/api/profile', update);
  }

  uploadProfilePhoto(file: File) {
    const form = new FormData();
    form.set('photo', file);

    return this.postForm<ProfilePhotoResponse>('/api/profile/photo', form);
  }

  dashboardEvents(limit = 24) {
    const params = new URLSearchParams({ limit: String(limit) });
    return this.get<DashboardEventsResponse>(
      `/api/dashboard/events?${params.toString()}`,
    );
  }

  event(eventId: string) {
    return this.get<EventRecord>(`/api/events/${encodeURIComponent(eventId)}`);
  }

  createEvent(draft: EventDraftRequest) {
    return this.post<EventRecord>('/api/events', draft);
  }

  updateEvent(eventId: string, draft: EventDraftRequest) {
    return this.put<EventRecord>(
      `/api/events/${encodeURIComponent(eventId)}`,
      draft,
    );
  }

  uploadEventCoverImage(eventId: string, file: File) {
    const form = new FormData();
    form.set('cover_image', file);

    return this.postForm<EventCoverImageResponse>(
      `/api/events/${encodeURIComponent(eventId)}/cover-image`,
      form,
    );
  }

  eventAttachments(eventId: string) {
    return this.get<EventAttachmentListResponse>(
      `/api/events/${encodeURIComponent(eventId)}/attachments`,
    );
  }

  uploadEventAttachment(eventId: string, file: File) {
    const form = new FormData();
    form.set('attachment', file);

    return this.postForm<EventAttachmentUploadResponse>(
      `/api/events/${encodeURIComponent(eventId)}/attachments`,
      form,
    );
  }

  eventAttachmentDownload(eventId: string, attachmentId: string) {
    return this.get<EventAttachmentDownloadResponse>(
      `/api/events/${encodeURIComponent(eventId)}/attachments/${encodeURIComponent(
        attachmentId,
      )}/download`,
    );
  }

  deleteEventAttachment(eventId: string, attachmentId: string) {
    return this.delete(
      `/api/events/${encodeURIComponent(eventId)}/attachments/${encodeURIComponent(
        attachmentId,
      )}`,
    );
  }

  sendEventInvitations(
    eventId: string,
    invitees: InvitationRecipientRequest[],
    message?: string | null,
  ) {
    return this.post<SendInvitationsResponse>(
      `/api/events/${encodeURIComponent(eventId)}/invitations`,
      {
        invitees,
        message: message?.trim() ? message.trim() : null,
      },
    );
  }

  eventAttendees(eventId: string) {
    return this.get<EventAttendeeListResponse>(
      `/api/events/${encodeURIComponent(eventId)}/attendees`,
    );
  }

  eventComments(eventId: string) {
    return this.get<EventCommentListResponse>(
      `/api/events/${encodeURIComponent(eventId)}/comments`,
    );
  }

  createEventComment(eventId: string, body: string) {
    return this.post<EventCommentCreateResponse>(
      `/api/events/${encodeURIComponent(eventId)}/comments`,
      { body },
    );
  }

  eventActivity(eventId: string) {
    return this.get<EventActivityListResponse>(
      `/api/events/${encodeURIComponent(eventId)}/activity`,
    );
  }

  updateEventRsvp(eventId: string, update: EventRsvpUpdateRequest) {
    return this.put<RsvpActionResponse>(
      `/api/events/${encodeURIComponent(eventId)}/rsvp`,
      update,
    );
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      credentials: 'include',
      headers: {
        Accept: 'application/json',
      },
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async post<T>(path: string, body?: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      credentials: 'include',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async put<T>(path: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'PUT',
      credentials: 'include',
      headers: {
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async postForm<T>(path: string, body: FormData): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      credentials: 'include',
      headers: {
        Accept: 'application/json',
      },
      body,
    });

    await this.ensureOk(response);

    return response.json() as Promise<T>;
  }

  private async delete(path: string): Promise<void> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'DELETE',
      credentials: 'include',
      headers: {
        Accept: 'application/json',
      },
    });

    await this.ensureOk(response);
  }

  private async ensureOk(response: Response) {
    if (response.ok) {
      return;
    }

    let message = `API request failed: ${response.status}`;
    let code: string | undefined;
    let details: unknown;
    try {
      const body = (await response.json()) as {
        error?: { code?: string; message?: string; details?: unknown };
      };
      code = body.error?.code;
      details = body.error?.details;
      message = body.error?.message ?? message;
    } catch {
      // Keep the status-based message when the response is not JSON.
    }

    throw new ApiError(response.status, message, code, details);
  }

  private authRedirectUrl(path: string, returnPath: string) {
    const url = new URL(path, this.baseUrl);
    url.searchParams.set('return_to', this.frontendReturnTo(returnPath));
    return url.href;
  }

  private frontendReturnTo(returnPath: string) {
    return new URL(returnPath, config.appBaseUrl).href;
  }
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
    public readonly code?: string,
    public readonly details?: unknown,
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export const apiClient = new ApiClient(config.apiBaseUrl);
