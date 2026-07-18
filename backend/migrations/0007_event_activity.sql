CREATE TABLE IF NOT EXISTS event_activity (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    actor_sub TEXT REFERENCES users(sub) ON DELETE SET NULL,
    activity_type TEXT NOT NULL,
    message TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS event_activity_event_id_idx ON event_activity(event_id);
CREATE INDEX IF NOT EXISTS event_activity_actor_sub_idx ON event_activity(actor_sub);
CREATE INDEX IF NOT EXISTS event_activity_created_at_idx ON event_activity(created_at);
