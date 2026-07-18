# alex-1883-gather

Managed Creator playground.

## Backend database

Set `DATABASE_URL` before running backend database tasks.

```bash
npm run db:migrate
```

The backend also runs embedded SQLx migrations on startup and verifies database connectivity through `/health`.

## Backend object storage

Configure the S3-compatible storage client with `OBJECT_STORAGE_ENDPOINT`, `OBJECT_STORAGE_REGION`, `OBJECT_STORAGE_BUCKET`, `OBJECT_STORAGE_ACCESS_KEY_ID`, `OBJECT_STORAGE_SECRET_ACCESS_KEY`, and `OBJECT_STORAGE_PREFIX`.

The backend storage module applies the configured prefix, supports object upload/download, and can generate presigned GET URLs for browser access.

## Backend email

Configure transactional email with `MCTAI_EMAIL_URL` and `MCTAI_EMAIL_APP_TOKEN`. The backend email module sends through the platform proxy only, skips sends when the proxy is not configured, and exposes a small template namespace for friendly transactional copy.

## Backend API

The Axum API shell exposes `/api`, `/health`, and `/api/health`. Feature routes should use the shared `api::error::ApiError` response type and `api::validation` helpers so JSON errors keep the same `{ "error": { "code", "message", "details" } }` shape.

## Backend users

User persistence is keyed by the verified platform auth subject. The `users` table stores identity and email-verification state, `user_identities` records the platform identity linkage, and `profiles` stores display name and profile photo object references for later account flows.

## Backend auth

Requests carrying an `mctai_session` cookie are verified against the platform JWKS, upserted into the local `users` table, and exposed to handlers through request extensions. The backend does not issue app JWTs or store passwords.

`POST /api/auth/register` records the current platform-authenticated user locally and sends a friendly registration email through the platform email proxy when configured. `GET /api/auth/verify` reports the platform email-verification status from the verified session.

`GET /api/auth/login` redirects to the platform login page with a safe frontend `return_to`; `POST /api/auth/login` returns the same URL as JSON for clients that need to render their own button.

`GET /api/auth/google` is a Google-login compatibility endpoint that also redirects through the platform auth service. `GET /api/auth/google/callback` does not exchange provider codes; it sends authenticated platform sessions back to a frontend page or restarts platform login when no `mctai_session` is present.

`POST /api/auth/password-reset/request` accepts an email address and sends a warm recovery email through the platform email proxy with a platform sign-in link. `POST /api/auth/password-reset/complete` is a legacy-compatible endpoint that returns the same platform sign-in path; the app does not store reset tokens or change passwords locally.

`GET /api/profile` returns the current user's read-only account settings and editable profile. `PUT /api/profile` updates only the authenticated user's profile fields.

`POST /api/profile/photo` accepts a multipart `photo` or `file` image upload, stores it in object storage, replaces the authenticated user's prior profile photo reference, and returns a short-lived access URL.

## Backend events

Events are owned by `users.sub`, carry title/description/scheduled time and optional cover-image object references, and support linked PDF attachment records with object-storage metadata.

Authenticated organizers can manage events through `GET /api/events`, `POST /api/events`, `GET /api/events/:event_id`, `PUT /api/events/:event_id`, and `DELETE /api/events/:event_id`. Create operations set the current platform-authenticated user as the organizer, and update/delete operations reject non-organizers. Event reads include the organizer plus users listed in `event_members` with `invited` or `accepted` status.

`POST /api/events/:event_id/cover-image` or `PUT /api/events/:event_id/cover-image` accepts a multipart `cover_image`, `image`, or `file` upload from the event organizer, stores a JPEG/PNG/WebP/GIF image in object storage, replaces the event's cover-image object key, deletes the prior cover image when possible, and returns the updated event plus a short-lived access URL.

Event organizers and invited/accepted members can manage PDF materials through `GET /api/events/:event_id/attachments`, `POST /api/events/:event_id/attachments`, and `GET /api/events/:event_id/attachments/:attachment_id/download`. Attachment uploads accept multipart `attachment` or `file` fields, require `application/pdf` content with a PDF header, store the file in object storage, and return attachment metadata plus a short-lived access URL. `DELETE /api/events/:event_id/attachments/:attachment_id` is limited to the organizer or the attachment uploader.

`GET /api/dashboard/events` returns the current user's upcoming organized, invited, or joined events ordered by start time for the dashboard. The optional `limit` query parameter is clamped from 1 to 100, and events with cover images include a short-lived `cover_image_url`.

Invitations are modeled in `event_invitations` with event, inviter, invitee, lifecycle status, and optional message fields. RSVP responses are stored in `event_rsvps` with casual `yes`, `no`, or `maybe` responses linked back to the invitation, event, and responding user.

`POST /api/events/:event_id/invitations` lets the event organizer invite up to 50 email recipients at a time. The endpoint creates or refreshes invitation records with response tokens, sends friendly accept/decline emails through the configured platform email proxy, and returns per-recipient delivery statuses of `sent`, `skipped`, `rate_limited`, or `failed`.

Invitees can inspect an invitation through `GET /api/invitations/:response_token`, accept or decline it through `POST /api/invitations/:response_token/response`, and update RSVP status through `PUT /api/events/:event_id/rsvp`. RSVP changes update invitation and event-member status, save a casual `yes`, `no`, or `maybe` RSVP, record an `event_activity` entry for later feed views, and send a confirmation email when the platform email proxy is configured. Organizers can review attendee status through `GET /api/events/:event_id/attendees`.

The backend starts an event reminder scheduler alongside the API server. Every 15 minutes it looks for events starting within the next 24 hours, claims unsent attendee reminders in `event_reminder_deliveries`, and sends warm reminder emails to the organizer plus accepted attendees through the platform email proxy. Missing email configuration skips delivery without crashing the job.
