CREATE TABLE IF NOT EXISTS event_members (
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    member_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'invited',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, member_sub),
    CHECK (status IN ('invited', 'accepted', 'declined', 'removed'))
);

CREATE INDEX IF NOT EXISTS event_members_member_sub_idx ON event_members(member_sub);
CREATE INDEX IF NOT EXISTS event_members_event_id_idx ON event_members(event_id);
