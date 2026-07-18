CREATE TABLE IF NOT EXISTS events (
    id TEXT PRIMARY KEY,
    owner_sub TEXT NOT NULL REFERENCES users (sub) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    starts_at TIMESTAMPTZ NOT NULL,
    timezone TEXT,
    cover_image_object_key TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_owner_sub ON events (owner_sub);
CREATE INDEX IF NOT EXISTS idx_events_starts_at ON events (starts_at);

CREATE TABLE IF NOT EXISTS event_attachments (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events (id) ON DELETE CASCADE,
    uploaded_by_sub TEXT NOT NULL REFERENCES users (sub) ON DELETE CASCADE,
    object_key TEXT NOT NULL,
    filename TEXT NOT NULL,
    content_type TEXT NOT NULL DEFAULT 'application/pdf',
    byte_size BIGINT NOT NULL CHECK (byte_size >= 0),
    page_count INTEGER CHECK (page_count IS NULL OR page_count >= 0),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, object_key)
);

CREATE INDEX IF NOT EXISTS idx_event_attachments_event_id ON event_attachments (event_id);
CREATE INDEX IF NOT EXISTS idx_event_attachments_uploaded_by_sub ON event_attachments (uploaded_by_sub);
