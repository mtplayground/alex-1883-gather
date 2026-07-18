ALTER TABLE event_invitations
    ADD COLUMN IF NOT EXISTS invitee_email TEXT,
    ADD COLUMN IF NOT EXISTS response_token TEXT;

ALTER TABLE event_invitations
    ALTER COLUMN invitee_sub DROP NOT NULL;

UPDATE event_invitations
SET invitee_email = invitee_sub
WHERE invitee_email IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS event_invitations_event_invitee_email_idx
    ON event_invitations (event_id, lower(invitee_email))
    WHERE invitee_email IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS event_invitations_response_token_idx
    ON event_invitations (response_token)
    WHERE response_token IS NOT NULL;
