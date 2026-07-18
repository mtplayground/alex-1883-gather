use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};

use crate::{
    api::{
        error::{ApiError, ApiResult},
        validation::{require_max_len, require_non_empty, ValidateRequest, ValidationErrors},
        AppState,
    },
    auth::CurrentUser,
    events::Event,
};

pub const ACTIVITY_RSVP_UPDATED: &str = "rsvp.updated";
pub const ACTIVITY_COMMENT_CREATED: &str = "comment.created";
pub const ACTIVITY_EVENT_EDITED: &str = "event.edited";

const COMMENT_BODY_MAX_CHARS: usize = 2_000;

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventComment {
    pub id: String,
    pub event_id: String,
    pub author_sub: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventCommentDraft {
    pub body: String,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventActivity {
    pub id: String,
    pub event_id: String,
    pub actor_sub: Option<String>,
    pub activity_type: String,
    pub message: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct EventCommentListResponse {
    pub comments: Vec<EventComment>,
}

#[derive(Debug, serde::Serialize)]
pub struct EventCommentCreateResponse {
    pub comment: EventComment,
}

#[derive(Debug, serde::Serialize)]
pub struct EventActivityListResponse {
    pub activity: Vec<EventActivity>,
}

#[derive(Clone)]
pub struct ActivityRepository {
    pool: PgPool,
}

impl ActivityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_comment(
        &self,
        event_id: &str,
        author_sub: &str,
        draft: &EventCommentDraft,
    ) -> Result<EventComment, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let body = draft.body.trim();

        sqlx::query_as::<_, EventComment>(
            r#"
            INSERT INTO event_comments (
                id,
                event_id,
                author_sub,
                body
            )
            VALUES ($1, $2, $3, $4)
            RETURNING
                id,
                event_id,
                author_sub,
                body,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(author_sub)
        .bind(body)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_comments(&self, event_id: &str) -> Result<Vec<EventComment>, sqlx::Error> {
        sqlx::query_as::<_, EventComment>(
            r#"
            SELECT
                id,
                event_id,
                author_sub,
                body,
                created_at,
                updated_at
            FROM event_comments
            WHERE event_id = $1
            ORDER BY created_at ASC, id ASC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn record_activity(
        &self,
        event_id: &str,
        actor_sub: Option<&str>,
        activity_type: &str,
        message: &str,
        payload: Value,
    ) -> Result<EventActivity, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, EventActivity>(
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
            RETURNING
                id,
                event_id,
                actor_sub,
                activity_type,
                message,
                payload,
                created_at
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(actor_sub)
        .bind(activity_type)
        .bind(message)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_activity(&self, event_id: &str) -> Result<Vec<EventActivity>, sqlx::Error> {
        sqlx::query_as::<_, EventActivity>(
            r#"
            SELECT
                id,
                event_id,
                actor_sub,
                activity_type,
                message,
                CASE
                    WHEN payload <> '{}'::jsonb THEN payload
                    ELSE metadata
                END AS payload,
                created_at
            FROM event_activity
            WHERE event_id = $1
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
    }
}

impl ValidateRequest for EventCommentDraft {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "body", &self.body);
        require_max_len(
            &mut errors,
            "body",
            self.body.trim(),
            COMMENT_BODY_MAX_CHARS,
        );

        errors.into_result()
    }
}

pub async fn list_event_comments(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<Json<EventCommentListResponse>> {
    let user = require_current_user(user)?;
    ensure_can_read_event(&state, &user, &event_id).await?;

    let comments = state
        .activity
        .list_comments(&event_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventCommentListResponse { comments }))
}

pub async fn create_event_comment(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    Json(draft): Json<EventCommentDraft>,
) -> ApiResult<(StatusCode, Json<EventCommentCreateResponse>)> {
    let user = require_current_user(user)?;
    draft.validate().map_err(ApiError::validation)?;
    ensure_can_read_event(&state, &user, &event_id).await?;

    let comment = state
        .activity
        .create_comment(&event_id, &user.sub, &draft)
        .await
        .map_err(ApiError::internal)?;
    let actor = display_name(&user);
    let message = format!("{actor} commented.");

    state
        .activity
        .record_activity(
            &event_id,
            Some(&user.sub),
            ACTIVITY_COMMENT_CREATED,
            &message,
            json!({
                "comment_id": &comment.id,
                "body_length": comment.body.chars().count(),
            }),
        )
        .await
        .map_err(ApiError::internal)?;

    Ok((
        StatusCode::CREATED,
        Json(EventCommentCreateResponse { comment }),
    ))
}

pub async fn list_event_activity(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<Json<EventActivityListResponse>> {
    let user = require_current_user(user)?;
    ensure_can_read_event(&state, &user, &event_id).await?;

    let activity = state
        .activity
        .list_activity(&event_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventActivityListResponse { activity }))
}

async fn ensure_can_read_event(
    state: &AppState,
    user: &CurrentUser,
    event_id: &str,
) -> ApiResult<Event> {
    let event = state
        .events
        .get(event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("event_not_found", "event not found"))?;

    if event.owner_sub == user.sub {
        return Ok(event);
    }

    let invited_member = state
        .events
        .is_invited_member(&event.id, &user.sub)
        .await
        .map_err(ApiError::internal)?;

    if invited_member {
        Ok(event)
    } else {
        Err(ApiError::forbidden(
            "event_forbidden",
            "event is not available to this user",
        ))
    }
}

fn display_name(user: &CurrentUser) -> String {
    user.name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| user.email.split('@').next())
        .unwrap_or("Someone")
        .to_string()
}

fn require_current_user(user: Option<Extension<CurrentUser>>) -> ApiResult<CurrentUser> {
    let Some(Extension(user)) = user else {
        return Err(ApiError::unauthorized(
            "not_authenticated",
            "valid platform session required",
        ));
    };

    Ok(user)
}

#[cfg(test)]
mod tests {
    use crate::api::validation::ValidateRequest;

    use super::{EventCommentDraft, ACTIVITY_COMMENT_CREATED};

    #[test]
    fn comment_body_must_not_be_blank() {
        let draft = EventCommentDraft {
            body: "   ".to_string(),
        };

        assert!(draft.validate().is_err());
    }

    #[test]
    fn comment_activity_type_is_stable() {
        assert_eq!(ACTIVITY_COMMENT_CREATED, "comment.created");
    }
}
