use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};

use crate::{
    activity::ACTIVITY_RSVP_UPDATED,
    api::{
        error::{ApiError, ApiResult},
        validation::{require_max_len, require_non_empty, ValidateRequest, ValidationErrors},
        AppState,
    },
    auth::CurrentUser,
    email::{templates, EmailError, EmailMessage},
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
    pub invitee_sub: Option<String>,
    pub invitee_email: Option<String>,
    #[serde(skip_serializing)]
    pub response_token: Option<String>,
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

#[derive(Clone, Debug, serde::Deserialize)]
pub struct SendInvitationsRequest {
    pub invitees: Vec<InvitationRecipient>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct InvitationRecipient {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SendInvitationsResponse {
    pub invitations: Vec<SentInvitation>,
}

#[derive(Debug, serde::Serialize)]
pub struct SentInvitation {
    pub invitation: EventInvitation,
    pub email_delivery: InvitationEmailDelivery,
}

#[derive(Debug, serde::Serialize)]
pub struct InvitationEmailDelivery {
    pub email: String,
    pub status: &'static str,
    pub id: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct InvitationResponseRequest {
    pub status: String,
    pub response: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventRsvpUpdateRequest {
    pub response: String,
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct InvitationDetailsResponse {
    pub invitation: EventInvitation,
    pub event: crate::events::Event,
}

#[derive(Debug, serde::Serialize)]
pub struct RsvpActionResponse {
    pub invitation: EventInvitation,
    pub rsvp: EventRsvp,
    pub email_delivery: InvitationEmailDelivery,
}

#[derive(Debug, serde::Serialize)]
pub struct EventAttendeeListResponse {
    pub attendees: Vec<EventAttendee>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventAttendee {
    pub invitation_id: String,
    pub event_id: String,
    pub invitee_sub: Option<String>,
    pub invitee_email: Option<String>,
    pub display_name: Option<String>,
    pub picture_url: Option<String>,
    pub invitation_status: String,
    pub rsvp_response: Option<String>,
    pub rsvp_note: Option<String>,
    pub responded_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
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
                invitee_email,
                response_token,
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

    pub async fn create_email_invitation(
        &self,
        event_id: &str,
        inviter_sub: &str,
        invitee_email: &str,
        message: Option<&str>,
        response_token: &str,
    ) -> Result<EventInvitation, sqlx::Error> {
        if let Some(existing) = self.find_by_event_email(event_id, invitee_email).await? {
            return self
                .refresh_email_invitation(&existing.id, inviter_sub, message, response_token)
                .await;
        }

        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, EventInvitation>(
            r#"
            INSERT INTO event_invitations (
                id,
                event_id,
                inviter_sub,
                invitee_email,
                response_token,
                status,
                message
            )
            VALUES ($1, $2, $3, $4, $5, 'invited', $6)
            RETURNING
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(inviter_sub)
        .bind(invitee_email)
        .bind(response_token)
        .bind(message)
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
                invitee_email,
                response_token,
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

    pub async fn get_by_response_token(
        &self,
        response_token: &str,
    ) -> Result<Option<EventInvitation>, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            SELECT
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            FROM event_invitations
            WHERE response_token = $1
            "#,
        )
        .bind(response_token)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_for_user(
        &self,
        event_id: &str,
        user_sub: &str,
        user_email: &str,
    ) -> Result<Option<EventInvitation>, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            SELECT
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            FROM event_invitations
            WHERE event_id = $1
                AND (
                    invitee_sub = $2
                    OR lower(invitee_email) = lower($3)
                )
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .bind(event_id)
        .bind(user_sub)
        .bind(user_email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn set_invitation_status_for_user(
        &self,
        invitation_id: &str,
        user_sub: &str,
        status: &str,
    ) -> Result<Option<EventInvitation>, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            UPDATE event_invitations
            SET
                invitee_sub = COALESCE(invitee_sub, $2),
                status = $3,
                updated_at = NOW()
            WHERE id = $1
                AND (invitee_sub IS NULL OR invitee_sub = $2)
            RETURNING
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            "#,
        )
        .bind(invitation_id)
        .bind(user_sub)
        .bind(status)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn upsert_event_member(
        &self,
        event_id: &str,
        user_sub: &str,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO event_members (event_id, member_sub, status)
            VALUES ($1, $2, $3)
            ON CONFLICT (event_id, member_sub)
            DO UPDATE SET
                status = EXCLUDED.status,
                updated_at = NOW()
            "#,
        )
        .bind(event_id)
        .bind(user_sub)
        .bind(status)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub async fn record_activity(
        &self,
        event_id: &str,
        actor_sub: &str,
        activity_type: &str,
        message: &str,
        metadata: Value,
    ) -> Result<(), sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO event_activity (
                id,
                event_id,
                actor_sub,
                activity_type,
                message,
                metadata,
                payload
            )
            VALUES ($1, $2, $3, $4, $5, $6, $6)
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(actor_sub)
        .bind(activity_type)
        .bind(message)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    pub async fn list_attendees(&self, event_id: &str) -> Result<Vec<EventAttendee>, sqlx::Error> {
        sqlx::query_as::<_, EventAttendee>(
            r#"
            SELECT
                event_invitations.id AS invitation_id,
                event_invitations.event_id,
                event_invitations.invitee_sub,
                event_invitations.invitee_email,
                COALESCE(profiles.display_name, users.name) AS display_name,
                users.picture_url,
                event_invitations.status AS invitation_status,
                event_rsvps.response AS rsvp_response,
                event_rsvps.note AS rsvp_note,
                event_rsvps.responded_at,
                GREATEST(event_invitations.updated_at, COALESCE(event_rsvps.updated_at, event_invitations.updated_at)) AS updated_at
            FROM event_invitations
            LEFT JOIN users ON users.sub = event_invitations.invitee_sub
            LEFT JOIN profiles ON profiles.user_sub = event_invitations.invitee_sub
            LEFT JOIN event_rsvps ON event_rsvps.invitation_id = event_invitations.id
            WHERE event_invitations.event_id = $1
            ORDER BY
                event_rsvps.responded_at DESC NULLS LAST,
                event_invitations.updated_at DESC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
    }

    async fn find_by_event_email(
        &self,
        event_id: &str,
        invitee_email: &str,
    ) -> Result<Option<EventInvitation>, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            SELECT
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            FROM event_invitations
            WHERE event_id = $1 AND lower(invitee_email) = lower($2)
            "#,
        )
        .bind(event_id)
        .bind(invitee_email)
        .fetch_optional(&self.pool)
        .await
    }

    async fn refresh_email_invitation(
        &self,
        invitation_id: &str,
        inviter_sub: &str,
        message: Option<&str>,
        response_token: &str,
    ) -> Result<EventInvitation, sqlx::Error> {
        sqlx::query_as::<_, EventInvitation>(
            r#"
            UPDATE event_invitations
            SET
                inviter_sub = $2,
                status = 'invited',
                message = $3,
                response_token = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                event_id,
                inviter_sub,
                invitee_sub,
                invitee_email,
                response_token,
                status,
                message,
                created_at,
                updated_at
            "#,
        )
        .bind(invitation_id)
        .bind(inviter_sub)
        .bind(message)
        .bind(response_token)
        .fetch_one(&self.pool)
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

pub async fn send_event_invitations(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    Json(request): Json<SendInvitationsRequest>,
) -> ApiResult<(StatusCode, Json<SendInvitationsResponse>)> {
    let user = require_current_user(user)?;
    request.validate().map_err(ApiError::validation)?;

    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("event_not_found", "event not found"))?;

    if event.owner_sub != user.sub {
        return Err(ApiError::forbidden(
            "event_forbidden",
            "Only the organizer can invite people to that event.",
        ));
    }

    let inviter_name = display_name(&user);
    let message = request
        .message
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty());
    let mut invitations = Vec::with_capacity(request.invitees.len());

    for invitee in request.invitees {
        let email = normalize_email(&invitee.email);
        let response_token = uuid::Uuid::new_v4().to_string();
        let invitation = state
            .invitations
            .create_email_invitation(&event.id, &user.sub, &email, message, &response_token)
            .await
            .map_err(ApiError::internal)?;
        let response_token = invitation
            .response_token
            .as_deref()
            .unwrap_or(response_token.as_str());
        let invite_url = invitation_response_url(&state.self_url, response_token);
        let delivery = send_invitation_email(
            &state,
            &email,
            &event.title,
            &inviter_name,
            &invite_url,
            message,
        )
        .await;

        invitations.push(SentInvitation {
            invitation,
            email_delivery: delivery,
        });
    }

    Ok((
        StatusCode::CREATED,
        Json(SendInvitationsResponse { invitations }),
    ))
}

pub async fn get_invitation_by_token(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(response_token): Path<String>,
) -> ApiResult<Json<InvitationDetailsResponse>> {
    let user = require_current_user(user)?;
    let invitation = state
        .invitations
        .get_by_response_token(&response_token)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("invitation_not_found", "invitation not found"))?;

    ensure_invitee_can_use_invitation(&invitation, &user)?;

    let event = state
        .events
        .get(&invitation.event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("event_not_found", "event not found"))?;

    Ok(Json(InvitationDetailsResponse { invitation, event }))
}

pub async fn respond_to_invitation(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(response_token): Path<String>,
    Json(request): Json<InvitationResponseRequest>,
) -> ApiResult<Json<RsvpActionResponse>> {
    let user = require_current_user(user)?;
    request.validate().map_err(ApiError::validation)?;

    let invitation = state
        .invitations
        .get_by_response_token(&response_token)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("invitation_not_found", "invitation not found"))?;

    ensure_invitee_can_use_invitation(&invitation, &user)?;
    ensure_invitation_active(&invitation)?;

    let response = response_for_invitation_status(&request.status, request.response.as_deref())?;

    complete_rsvp_action(
        &state,
        &user,
        &invitation,
        &request.status,
        response,
        request.note,
    )
    .await
    .map(Json)
}

pub async fn update_event_rsvp(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    Json(request): Json<EventRsvpUpdateRequest>,
) -> ApiResult<Json<RsvpActionResponse>> {
    let user = require_current_user(user)?;
    request.validate().map_err(ApiError::validation)?;

    let invitation = state
        .invitations
        .find_for_user(&event_id, &user.sub, &user.email)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("invitation_not_found", "invitation not found"))?;

    ensure_invitee_can_use_invitation(&invitation, &user)?;
    ensure_invitation_active(&invitation)?;

    let invitation_status = status_for_rsvp_response(&request.response);

    complete_rsvp_action(
        &state,
        &user,
        &invitation,
        invitation_status,
        &request.response,
        request.note,
    )
    .await
    .map(Json)
}

pub async fn list_event_attendees(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<Json<EventAttendeeListResponse>> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("event_not_found", "event not found"))?;

    let can_read = event.owner_sub == user.sub
        || state
            .events
            .is_invited_member(&event.id, &user.sub)
            .await
            .map_err(ApiError::internal)?;

    if !can_read {
        return Err(ApiError::forbidden(
            "event_forbidden",
            "That attendee list is private to the organizer and guest list.",
        ));
    }

    let attendees = state
        .invitations
        .list_attendees(&event_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventAttendeeListResponse { attendees }))
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

impl ValidateRequest for SendInvitationsRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        if self.invitees.is_empty() {
            errors.push("invitees", "must include at least one recipient");
        }

        if self.invitees.len() > 50 {
            errors.push("invitees", "must include at most 50 recipients");
        }

        for invitee in &self.invitees {
            validate_email(&mut errors, "invitees.email", &invitee.email);

            if let Some(name) = &invitee.name {
                require_max_len(&mut errors, "invitees.name", name, 120);
            }
        }

        if let Some(message) = &self.message {
            require_max_len(&mut errors, "message", message, 1000);
        }

        errors.into_result()
    }
}

impl ValidateRequest for InvitationResponseRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        if !matches!(
            self.status.as_str(),
            INVITATION_STATUS_ACCEPTED | INVITATION_STATUS_DECLINED
        ) {
            errors.push("status", "must be accepted or declined");
        }

        if let Some(response) = &self.response {
            if !is_rsvp_response(response) {
                errors.push("response", "must be yes, no, or maybe");
            }
        }

        if self.status == INVITATION_STATUS_DECLINED
            && self
                .response
                .as_deref()
                .is_some_and(|response| response != RSVP_NO)
        {
            errors.push("response", "declined invitations must use no");
        }

        if self.status == INVITATION_STATUS_ACCEPTED
            && self
                .response
                .as_deref()
                .is_some_and(|response| response == RSVP_NO)
        {
            errors.push("response", "accepted invitations must use yes or maybe");
        }

        if let Some(note) = &self.note {
            require_max_len(&mut errors, "note", note, 1000);
        }

        errors.into_result()
    }
}

impl ValidateRequest for EventRsvpUpdateRequest {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

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

fn require_current_user(user: Option<Extension<CurrentUser>>) -> ApiResult<CurrentUser> {
    let Some(Extension(user)) = user else {
        return Err(ApiError::unauthorized(
            "not_authenticated",
            "Your session has expired. Sign in again to keep going.",
        ));
    };

    Ok(user)
}

async fn complete_rsvp_action(
    state: &AppState,
    user: &CurrentUser,
    invitation: &EventInvitation,
    invitation_status: &str,
    response: &str,
    note: Option<String>,
) -> ApiResult<RsvpActionResponse> {
    let event = state
        .events
        .get(&invitation.event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("event_not_found", "event not found"))?;
    let member_status = member_status_for_invitation_status(invitation_status);
    let note = note
        .map(|note| note.trim().to_string())
        .filter(|note| !note.is_empty());
    let invitation = state
        .invitations
        .set_invitation_status_for_user(&invitation.id, &user.sub, invitation_status)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::forbidden(
                "invitation_forbidden",
                "That invite belongs to another guest.",
            )
        })?;

    state
        .invitations
        .upsert_event_member(&event.id, &user.sub, member_status)
        .await
        .map_err(ApiError::internal)?;

    let rsvp = state
        .invitations
        .upsert_rsvp(
            &user.sub,
            &EventRsvpDraft {
                invitation_id: invitation.id.clone(),
                event_id: event.id.clone(),
                response: response.to_string(),
                note: note.clone(),
            },
        )
        .await
        .map_err(ApiError::internal)?;

    record_rsvp_activity(state, user, &event.id, response, note.as_deref()).await?;
    let email_delivery =
        send_rsvp_confirmation_email(state, &user.email, &event.title, response, note.as_deref())
            .await;

    Ok(RsvpActionResponse {
        invitation,
        rsvp,
        email_delivery,
    })
}

async fn record_rsvp_activity(
    state: &AppState,
    user: &CurrentUser,
    event_id: &str,
    response: &str,
    note: Option<&str>,
) -> ApiResult<()> {
    let actor = display_name(user);
    let phrase = rsvp_activity_phrase(response);
    let message = format!("{actor} {phrase}.");

    state
        .invitations
        .record_activity(
            event_id,
            &user.sub,
            ACTIVITY_RSVP_UPDATED,
            &message,
            json!({
                "response": response,
                "note": note,
            }),
        )
        .await
        .map_err(ApiError::internal)
}

async fn send_invitation_email(
    state: &AppState,
    email: &str,
    event_title: &str,
    inviter_name: &str,
    invite_url: &str,
    message: Option<&str>,
) -> InvitationEmailDelivery {
    let subject = format!("You've got a seat: {event_title}");
    let html = templates::event_invitation_html(event_title, inviter_name, invite_url, message);
    let text = templates::event_invitation_text(event_title, inviter_name, invite_url, message);
    let email_message = EmailMessage::new(email.to_string(), subject)
        .html(html)
        .text(text);

    match state.email.send(email_message).await {
        Ok(Some(dispatch)) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "sent",
            id: Some(dispatch.id),
            message: None,
        },
        Ok(None) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "skipped",
            id: None,
            message: Some("email proxy is not configured".to_string()),
        },
        Err(EmailError::RateLimited) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "rate_limited",
            id: None,
            message: Some("try again shortly".to_string()),
        },
        Err(error) => {
            tracing::error!(%error, invitee_email = %email, "invitation email failed");
            InvitationEmailDelivery {
                email: email.to_string(),
                status: "failed",
                id: None,
                message: Some("invitation was created, but email could not be sent".to_string()),
            }
        }
    }
}

async fn send_rsvp_confirmation_email(
    state: &AppState,
    email: &str,
    event_title: &str,
    response: &str,
    note: Option<&str>,
) -> InvitationEmailDelivery {
    let label = rsvp_response_label(response);
    let subject = format!("RSVP saved: {event_title}");
    let html = templates::rsvp_confirmation_html(event_title, label, note);
    let text = templates::rsvp_confirmation_text(event_title, label, note);
    let email_message = EmailMessage::new(email.to_string(), subject)
        .html(html)
        .text(text);

    match state.email.send(email_message).await {
        Ok(Some(dispatch)) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "sent",
            id: Some(dispatch.id),
            message: None,
        },
        Ok(None) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "skipped",
            id: None,
            message: Some("email proxy is not configured".to_string()),
        },
        Err(EmailError::RateLimited) => InvitationEmailDelivery {
            email: email.to_string(),
            status: "rate_limited",
            id: None,
            message: Some("try again shortly".to_string()),
        },
        Err(error) => {
            tracing::error!(%error, invitee_email = %email, "rsvp confirmation email failed");
            InvitationEmailDelivery {
                email: email.to_string(),
                status: "failed",
                id: None,
                message: Some("rsvp was saved, but email could not be sent".to_string()),
            }
        }
    }
}

fn display_name(user: &CurrentUser) -> String {
    user.name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| user.email.split('@').next())
        .unwrap_or("The organizer")
        .to_string()
}

fn invitation_response_url(self_url: &str, response_token: &str) -> String {
    format!(
        "{}/invite/{}",
        self_url.trim_end_matches('/'),
        response_token
    )
}

fn validate_email(errors: &mut ValidationErrors, field: &'static str, email: &str) {
    require_non_empty(errors, field, email);
    require_max_len(errors, field, email, 320);

    let trimmed = email.trim();
    if !trimmed.is_empty()
        && (!trimmed.contains('@') || trimmed.starts_with('@') || trimmed.ends_with('@'))
    {
        errors.push(field, "must be a valid email address");
    }
}

fn normalize_email(email: &str) -> String {
    email.trim().to_ascii_lowercase()
}

fn ensure_invitee_can_use_invitation(
    invitation: &EventInvitation,
    user: &CurrentUser,
) -> ApiResult<()> {
    let sub_matches = invitation
        .invitee_sub
        .as_deref()
        .is_some_and(|sub| sub == user.sub);
    let email_matches = invitation
        .invitee_email
        .as_deref()
        .is_some_and(|email| email.eq_ignore_ascii_case(&user.email));

    if sub_matches || email_matches {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "invitation_forbidden",
        "That invite belongs to another guest.",
    ))
}

fn ensure_invitation_active(invitation: &EventInvitation) -> ApiResult<()> {
    if invitation.status == INVITATION_STATUS_CANCELLED {
        return Err(ApiError::bad_request(
            "invitation_cancelled",
            "cancelled invitations cannot be updated",
        ));
    }

    Ok(())
}

fn response_for_invitation_status<'a>(
    status: &str,
    response: Option<&'a str>,
) -> ApiResult<&'a str> {
    match (status, response) {
        (INVITATION_STATUS_ACCEPTED, Some(response)) => Ok(response),
        (INVITATION_STATUS_ACCEPTED, None) => Ok(RSVP_YES),
        (INVITATION_STATUS_DECLINED, Some(response)) => Ok(response),
        (INVITATION_STATUS_DECLINED, None) => Ok(RSVP_NO),
        _ => Err(ApiError::bad_request(
            "invalid_invitation_status",
            "status must be accepted or declined",
        )),
    }
}

fn status_for_rsvp_response(response: &str) -> &'static str {
    if response == RSVP_NO {
        INVITATION_STATUS_DECLINED
    } else {
        INVITATION_STATUS_ACCEPTED
    }
}

fn member_status_for_invitation_status(status: &str) -> &'static str {
    if status == INVITATION_STATUS_DECLINED {
        "declined"
    } else {
        "accepted"
    }
}

fn rsvp_response_label(response: &str) -> &'static str {
    match response {
        RSVP_YES => "Yes",
        RSVP_NO => "No",
        RSVP_MAYBE => "Maybe",
        _ => "Updated",
    }
}

fn rsvp_activity_phrase(response: &str) -> &'static str {
    match response {
        RSVP_YES => "is in",
        RSVP_NO => "can't make it",
        RSVP_MAYBE => "might make it",
        _ => "updated their RSVP",
    }
}

#[cfg(test)]
mod tests {
    use crate::api::validation::ValidateRequest;

    use super::{
        invitation_response_url, is_invitation_status, is_rsvp_response,
        member_status_for_invitation_status, response_for_invitation_status, rsvp_activity_phrase,
        status_for_rsvp_response, EventInvitationDraft, EventInvitationStatusUpdate,
        EventRsvpDraft, EventRsvpUpdateRequest, InvitationRecipient, InvitationResponseRequest,
        SendInvitationsRequest, INVITATION_STATUS_ACCEPTED, INVITATION_STATUS_DECLINED, RSVP_MAYBE,
        RSVP_NO, RSVP_YES,
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
    fn rsvp_activity_phrases_read_like_feed_items() {
        assert_eq!(rsvp_activity_phrase(RSVP_YES), "is in");
        assert_eq!(rsvp_activity_phrase(RSVP_NO), "can't make it");
        assert_eq!(rsvp_activity_phrase(RSVP_MAYBE), "might make it");
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

    #[test]
    fn send_invitations_requires_valid_recipients() {
        let request = SendInvitationsRequest {
            invitees: vec![InvitationRecipient {
                email: "not-an-email".to_string(),
                name: None,
            }],
            message: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn send_invitations_accepts_email_and_message() {
        let request = SendInvitationsRequest {
            invitees: vec![InvitationRecipient {
                email: "friend@example.com".to_string(),
                name: Some("Friend".to_string()),
            }],
            message: Some("Bring ideas.".to_string()),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn invitation_response_url_trims_base_slash() {
        assert_eq!(
            invitation_response_url("https://example.com/", "token-1"),
            "https://example.com/invite/token-1"
        );
    }

    #[test]
    fn invitation_response_defaults_rsvp_from_accept_decline() {
        assert_eq!(
            response_for_invitation_status(INVITATION_STATUS_ACCEPTED, None).unwrap(),
            RSVP_YES
        );
        assert_eq!(
            response_for_invitation_status(INVITATION_STATUS_DECLINED, None).unwrap(),
            RSVP_NO
        );
    }

    #[test]
    fn rsvp_response_maps_to_invitation_and_member_status() {
        assert_eq!(
            status_for_rsvp_response(RSVP_YES),
            INVITATION_STATUS_ACCEPTED
        );
        assert_eq!(
            status_for_rsvp_response(RSVP_MAYBE),
            INVITATION_STATUS_ACCEPTED
        );
        assert_eq!(
            status_for_rsvp_response(RSVP_NO),
            INVITATION_STATUS_DECLINED
        );
        assert_eq!(
            member_status_for_invitation_status(INVITATION_STATUS_DECLINED),
            "declined"
        );
    }

    #[test]
    fn invitation_response_validation_rejects_conflicting_status() {
        let request = InvitationResponseRequest {
            status: INVITATION_STATUS_DECLINED.to_string(),
            response: Some(RSVP_YES.to_string()),
            note: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn event_rsvp_update_accepts_maybe() {
        let request = EventRsvpUpdateRequest {
            response: RSVP_MAYBE.to_string(),
            note: Some("Still checking travel.".to_string()),
        };

        assert!(request.validate().is_ok());
    }
}
