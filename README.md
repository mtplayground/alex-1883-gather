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
