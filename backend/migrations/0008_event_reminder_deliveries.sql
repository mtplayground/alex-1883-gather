CREATE TABLE IF NOT EXISTS event_reminder_deliveries (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    recipient_key TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    reminder_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    email_dispatch_id TEXT,
    message TEXT,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, recipient_key, reminder_kind),
    CHECK (status IN ('pending', 'sent', 'skipped', 'rate_limited', 'failed'))
);

CREATE INDEX IF NOT EXISTS event_reminder_deliveries_event_id_idx
    ON event_reminder_deliveries(event_id);

CREATE INDEX IF NOT EXISTS event_reminder_deliveries_recipient_key_idx
    ON event_reminder_deliveries(recipient_key);

CREATE INDEX IF NOT EXISTS event_reminder_deliveries_status_idx
    ON event_reminder_deliveries(status);
