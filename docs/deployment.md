# Bare Deployment

This project ships as two plain artifacts: a Rust API binary and a static Vite frontend directory. It does not require Docker, container orchestration, or CI/CD.

## Prerequisites

- Node.js 20 or newer
- Rust stable toolchain
- PostgreSQL reachable through `DATABASE_URL`
- S3-compatible object storage credentials
- A reverse proxy or static file server for `frontend/dist`
- Platform auth variables supplied by Ideavibes:
  - `MCTAI_AUTH_URL=https://auth.mctai.app`
  - `MCTAI_AUTH_APP_TOKEN=app_alex-1883-gather-f41811`
  - `MCTAI_AUTH_JWKS_URL=https://auth.mctai.app/.well-known/jwks.json`

## Environment

Start from the checked-in template and fill deployment-specific values:

```bash
cp .env.example .env.production
```

Required backend variables:

- `HOST` and `PORT`
- `SELF_URL`
- `DATABASE_URL`
- `OBJECT_STORAGE_ENDPOINT`
- `OBJECT_STORAGE_REGION`
- `OBJECT_STORAGE_BUCKET`
- `OBJECT_STORAGE_ACCESS_KEY_ID`
- `OBJECT_STORAGE_SECRET_ACCESS_KEY`
- `OBJECT_STORAGE_PREFIX`
- `MCTAI_AUTH_URL`
- `MCTAI_AUTH_APP_TOKEN`
- `MCTAI_AUTH_JWKS_URL`

Optional backend email variables:

- `MCTAI_EMAIL_URL`
- `MCTAI_EMAIL_APP_TOKEN`
- `MCTAI_EMAIL_SENDER_NAME`

If the email proxy variables are blank, transactional emails are skipped without crashing requests or background reminder jobs.

Frontend variables are build-time values. Set `VITE_API_BASE_URL`, `VITE_APP_BASE_URL`, `VITE_MCTAI_AUTH_URL`, and `VITE_MCTAI_AUTH_APP_TOKEN` before running the frontend build.

## Verify Configuration

Load the production env file and run the config check before migrations or startup:

```bash
set -a
. ./.env.production
set +a
npm run config:check
```

The command exits non-zero when a required variable is missing, empty, malformed, or when a port value cannot be parsed.

## Build

Install dependencies and build release artifacts:

```bash
npm install
npm run build:release
```

Outputs:

- Backend binary: `target/release/alex-1883-gather-backend`
- Frontend static files: `frontend/dist`

## Database Migration

Run migrations with the same `DATABASE_URL` used by the backend:

```bash
npm run db:migrate
```

The backend also runs embedded SQLx migrations on startup, so repeated runs are safe.

## Run

Start the API process with the environment already loaded:

```bash
./target/release/alex-1883-gather-backend
```

Serve `frontend/dist` from a static web server or reverse proxy. The frontend should be reachable at `SELF_URL`; API requests should route to `VITE_API_BASE_URL`.

Example reverse-proxy layout:

- `https://example.mctai.app` serves `frontend/dist`
- `https://api.example.mctai.app` proxies to `HOST:PORT`

Health checks:

```bash
curl -fsS "$VITE_API_BASE_URL/health"
curl -fsS "$VITE_API_BASE_URL/api/health"
```

## Upgrade Procedure

1. Pull the new source version.
2. Rebuild with `npm run build:release`.
3. Run `npm run config:check`.
4. Run `npm run db:migrate`.
5. Replace the deployed `frontend/dist` directory.
6. Restart the `alex-1883-gather-backend` process.
7. Confirm both health endpoints return success.
