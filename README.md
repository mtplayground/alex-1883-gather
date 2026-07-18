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
