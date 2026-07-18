use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::api::validation::{
    require_max_len, require_non_empty, ValidateRequest, ValidationErrors,
};

pub const INVITATION_STATUS_INVITED: &str = "invited";
pub const INVITATION_STATUS_ACCEPTED: &str = "accepted";
pub const INVITATION_STATUS_DECLINED: &str = "declined";
pub const INVITATION_STATUS_CANCELLED: &str = "cancelled";

pub const RSVP_YES: &str = "yes";
pub const RSVP_NO: &str = "no";
pub const RSVP_MAYBE: &str = "maybe";

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventInvitation {
    pub id: String,
    pub event_id: String,
    pub inviter_sub: String,
    pub invitee_sub: String,
    pub status: String,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventInvitationDraft {
    pub event_id: String,
    pub invitee_sub: String,
    pub message: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventInvitationStatusUpdate {
    pub status: String,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventRsvp {
    pub id: String,
    pub invitation_id: String,
    pub event_id: String,
    pub user_sub: String,
    pub response: String,
    pub note: Option<String>,
    pub responded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventRsvpDraft {
    pub invitation_id: String,
    pub event_id: String,
    pub response: String,
    pub note: Option<String>,
}

#[derive(Clone)]
pub struct InvitationRepository {
    pool: PgPool,
}

impl InvitationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_invitation(
        &self,
        inviter_sub: &str,
        draft: &EventInvitationDraft,
    ) -> Result<EventInvitation, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, EventInvitation>(
            r#"
            INSERT INTO event_invitations (
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                status,
                message
            )
            VALUES ($1, $2, $3, $4, 'invited', $5)
            RETURNING
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                status,
                message,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(&draft.event_id)
        .bind(inviter_sub)
        .bind(&draft.invitee_sub)
        .bind(&draft.message)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_invitation_status(
        &self,
        invitation_id: &str,
        status: &str,
    ) -> Result<Option<EventInvitation>, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            UPDATE event_invitations
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                status,
                message,
                created_at,
                updated_at
            "#,
        )
        .bind(invitation_id)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn upsert_rsvp(
        &self,
        user_sub: &str,
        draft: &EventRsvpDraft,
    ) -> Result<EventRsvp, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, EventRsvp>(
            r#"
            INSERT INTO event_rsvps (
                id,
                invitation_id,
                event_id,
                user_sub,
                response,
                note
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (event_id, user_sub)
            DO UPDATE SET
                invitation_id = EXCLUDED.invitation_id,
                response = EXCLUDED.response,
                note = EXCLUDED.note,
                responded_at = NOW(),
                updated_at = NOW()
            RETURNING
                id,
                invitation_id,
                event_id,
                user_sub,
                response,
                note,
                responded_at,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(&draft.invitation_id)
        .bind(&draft.event_id)
        .bind(user_sub)
        .bind(&draft.response)
        .bind(&draft.note)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_event_rsvps(&self, event_id: &str) -> Result<Vec<EventRsvp>, sqlx::Error> {
        sqlx::query_as::<_, EventRsvp>(
            r#"
            SELECT
                id,
                invitation_id,
                event_id,
                user_sub,
                response,
                note,
                responded_at,
                created_at,
                updated_at
            FROM event_rsvps
            WHERE event_id = $1
            ORDER BY responded_at DESC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
    }
}

impl ValidateRequest for EventInvitationDraft {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "event_id", &self.event_id);
        require_max_len(&mut errors, "event_id", &self.event_id, 120);
        require_non_empty(&mut errors, "invitee_sub", &self.invitee_sub);
        require_max_len(&mut errors, "invitee_sub", &self.invitee_sub, 255);

        if let Some(message) = &self.message {
            require_max_len(&mut errors, "message", message, 1000);
        }

        errors.into_result()
    }
}

impl ValidateRequest for EventInvitationStatusUpdate {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        if !is_invitation_status(&self.status) {
            errors.push(
                "status",
                "must be invited, accepted, declined, or cancelled",
            );
        }

        errors.into_result()
    }
}

impl ValidateRequest for EventRsvpDraft {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "invitation_id", &self.invitation_id);
        require_max_len(&mut errors, "invitation_id", &self.invitation_id, 120);
        require_non_empty(&mut errors, "event_id", &self.event_id);
        require_max_len(&mut errors, "event_id", &self.event_id, 120);

        if !is_rsvp_response(&self.response) {
            errors.push("response", "must be yes, no, or maybe");
        }

        if let Some(note) = &self.note {
            require_max_len(&mut errors, "note", note, 1000);
        }

        errors.into_result()
    }
}

pub fn is_invitation_status(status: &str) -> bool {
    matches!(
        status,
        INVITATION_STATUS_INVITED
            | INVITATION_STATUS_ACCEPTED
            | INVITATION_STATUS_DECLINED
            | INVITATION_STATUS_CANCELLED
    )
}

pub fn is_rsvp_response(response: &str) -> bool {
    matches!(response, RSVP_YES | RSVP_NO | RSVP_MAYBE)
}

#[cfg(test)]
mod tests {
    use crate::api::validation::ValidateRequest;

    use super::{
        is_invitation_status, is_rsvp_response, EventInvitationDraft, EventInvitationStatusUpdate,
        EventRsvpDraft,
    };

    #[test]
    fn invitation_status_allows_expected_lifecycle_values() {
        assert!(is_invitation_status("invited"));
        assert!(is_invitation_status("accepted"));
        assert!(is_invitation_status("declined"));
        assert!(is_invitation_status("cancelled"));
        assert!(!is_invitation_status("maybe"));
    }

    #[test]
    fn rsvp_response_uses_casual_values() {
        assert!(is_rsvp_response("yes"));
        assert!(is_rsvp_response("no"));
        assert!(is_rsvp_response("maybe"));
        assert!(!is_rsvp_response("accepted"));
    }

    #[test]
    fn invitation_draft_requires_event_and_invitee() {
        let draft = EventInvitationDraft {
            event_id: "".to_string(),
            invitee_sub: " ".to_string(),
            message: None,
        };

        assert!(draft.validate().is_err());
    }

    #[test]
    fn status_update_rejects_unknown_status() {
        let update = EventInvitationStatusUpdate {
            status: "later".to_string(),
        };

        assert!(update.validate().is_err());
    }

    #[test]
    fn rsvp_draft_rejects_non_casual_response() {
        let draft = EventRsvpDraft {
            invitation_id: "invitation-1".to_string(),
            event_id: "event-1".to_string(),
            response: "accepted".to_string(),
            note: None,
        };

        assert!(draft.validate().is_err());
    }
}
