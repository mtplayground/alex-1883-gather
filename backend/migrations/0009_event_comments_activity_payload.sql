CREATE TABLE IF NOT EXISTS event_comments (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    author_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS event_comments_event_id_idx ON event_comments(event_id);
CREATE INDEX IF NOT EXISTS event_comments_author_sub_idx ON event_comments(author_sub);
CREATE INDEX IF NOT EXISTS event_comments_created_at_idx ON event_comments(created_at);

ALTER TABLE event_activity
    ADD COLUMN IF NOT EXISTS payload JSONB NOT NULL DEFAULT '{}'::jsonb;

UPDATE event_activity
SET payload = metadata
WHERE payload = '{}'::jsonb
    AND metadata <> '{}'::jsonb;

CREATE INDEX IF NOT EXISTS event_activity_activity_type_idx ON event_activity(activity_type);
