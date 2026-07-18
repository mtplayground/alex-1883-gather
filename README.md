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
