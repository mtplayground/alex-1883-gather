# alex-1883-gather

Managed Creator playground.

## Backend database

Set `DATABASE_URL` before running backend database tasks.

```bash
npm run db:migrate
```

The backend also runs embedded SQLx migrations on startup and verifies database connectivity through `/health`.
