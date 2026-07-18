CREATE TABLE IF NOT EXISTS event_invitations (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    inviter_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    invitee_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'invited',
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, invitee_sub),
    CHECK (status IN ('invited', 'accepted', 'declined', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS event_invitations_event_id_idx ON event_invitations(event_id);
CREATE INDEX IF NOT EXISTS event_invitations_inviter_sub_idx ON event_invitations(inviter_sub);
CREATE INDEX IF NOT EXISTS event_invitations_invitee_sub_idx ON event_invitations(invitee_sub);
CREATE INDEX IF NOT EXISTS event_invitations_status_idx ON event_invitations(status);

CREATE TABLE IF NOT EXISTS event_rsvps (
    id TEXT PRIMARY KEY,
    invitation_id TEXT NOT NULL UNIQUE REFERENCES event_invitations(id) ON DELETE CASCADE,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_sub TEXT NOT NULL REFERENCES users(sub) ON DELETE CASCADE,
    response TEXT NOT NULL,
    note TEXT,
    responded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (event_id, user_sub),
    CHECK (response IN ('yes', 'no', 'maybe'))
);

CREATE INDEX IF NOT EXISTS event_rsvps_event_id_idx ON event_rsvps(event_id);
CREATE INDEX IF NOT EXISTS event_rsvps_user_sub_idx ON event_rsvps(user_sub);
CREATE INDEX IF NOT EXISTS event_rsvps_response_idx ON event_rsvps(response);
