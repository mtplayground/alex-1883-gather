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
struct EventCommentRow {
    pub id: String,
    pub event_id: String,
    pub author_sub: String,
    pub author_email: String,
    pub author_name: Option<String>,
    pub author_picture_url: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EventComment {
    pub id: String,
    pub event_id: String,
    pub author: EventCommentAuthor,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EventCommentAuthor {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture_url: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventCommentDraft {
    pub body: String,
}

#[derive(Clone, Debug, FromRow)]
struct EventActivityRow {
    pub id: String,
    pub event_id: String,
    pub actor_sub: Option<String>,
    pub actor_email: Option<String>,
    pub actor_name: Option<String>,
    pub actor_picture_url: Option<String>,
    pub activity_type: String,
    pub message: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EventActivity {
    pub id: String,
    pub event_id: String,
    pub actor: Option<EventActivityActor>,
    pub activity_type: String,
    pub message: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EventActivityActor {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub picture_url: Option<String>,
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

        sqlx::query_as::<_, EventCommentRow>(
            r#"
            WITH inserted AS (
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
            )
            SELECT
                inserted.id,
                inserted.event_id,
                inserted.author_sub,
                users.email AS author_email,
                COALESCE(profiles.display_name, users.name) AS author_name,
                users.picture_url AS author_picture_url,
                inserted.body,
                inserted.created_at,
                inserted.updated_at
            FROM inserted
            JOIN users ON users.sub = inserted.author_sub
            LEFT JOIN profiles ON profiles.user_sub = inserted.author_sub
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(author_sub)
        .bind(body)
        .fetch_one(&self.pool)
        .await
        .map(EventComment::from)
    }

    pub async fn list_comments(&self, event_id: &str) -> Result<Vec<EventComment>, sqlx::Error> {
        let rows = sqlx::query_as::<_, EventCommentRow>(
            r#"
            SELECT
                event_comments.id,
                event_comments.event_id,
                event_comments.author_sub,
                users.email AS author_email,
                COALESCE(profiles.display_name, users.name) AS author_name,
                users.picture_url AS author_picture_url,
                event_comments.body,
                event_comments.created_at,
                event_comments.updated_at
            FROM event_comments
            JOIN users ON users.sub = event_comments.author_sub
            LEFT JOIN profiles ON profiles.user_sub = event_comments.author_sub
            WHERE event_comments.event_id = $1
            ORDER BY event_comments.created_at ASC, event_comments.id ASC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(EventComment::from).collect())
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

        sqlx::query_as::<_, EventActivityRow>(
            r#"
            WITH inserted AS (
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
            )
            SELECT
                inserted.id,
                inserted.event_id,
                inserted.actor_sub,
                users.email AS actor_email,
                COALESCE(profiles.display_name, users.name) AS actor_name,
                users.picture_url AS actor_picture_url,
                inserted.activity_type,
                inserted.message,
                inserted.payload,
                inserted.created_at
            FROM inserted
            LEFT JOIN users ON users.sub = inserted.actor_sub
            LEFT JOIN profiles ON profiles.user_sub = inserted.actor_sub
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
        .map(EventActivity::from)
    }

    pub async fn list_activity(&self, event_id: &str) -> Result<Vec<EventActivity>, sqlx::Error> {
        let rows = sqlx::query_as::<_, EventActivityRow>(
            r#"
            SELECT
                event_activity.id,
                event_activity.event_id,
                event_activity.actor_sub,
                users.email AS actor_email,
                COALESCE(profiles.display_name, users.name) AS actor_name,
                users.picture_url AS actor_picture_url,
                event_activity.activity_type,
                event_activity.message,
                CASE
                    WHEN event_activity.payload <> '{}'::jsonb THEN event_activity.payload
                    ELSE event_activity.metadata
                END AS payload,
                event_activity.created_at
            FROM event_activity
            LEFT JOIN users ON users.sub = event_activity.actor_sub
            LEFT JOIN profiles ON profiles.user_sub = event_activity.actor_sub
            WHERE event_activity.event_id = $1
            ORDER BY event_activity.created_at DESC, event_activity.id DESC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(EventActivity::from).collect())
    }
}

impl From<EventCommentRow> for EventComment {
    fn from(row: EventCommentRow) -> Self {
        Self {
            id: row.id,
            event_id: row.event_id,
            author: EventCommentAuthor {
                sub: row.author_sub,
                email: row.author_email,
                name: row.author_name,
                picture_url: row.author_picture_url,
            },
            body: row.body,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<EventActivityRow> for EventActivity {
    fn from(row: EventActivityRow) -> Self {
        Self {
            id: row.id,
            event_id: row.event_id,
            actor: row.actor_sub.map(|sub| EventActivityActor {
                sub,
                email: row.actor_email,
                name: row.actor_name,
                picture_url: row.actor_picture_url,
            }),
            activity_type: row.activity_type,
            message: row.message,
            payload: row.payload,
            created_at: row.created_at,
        }
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
    let message = format!("{actor} added a comment.");

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
            "That event is private to its organizer and guest list.",
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
            "Your session has expired. Sign in again to keep going.",
        ));
    };

    Ok(user)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;

    use crate::api::validation::ValidateRequest;

    use super::{
        EventActivity, EventActivityRow, EventComment, EventCommentDraft, EventCommentRow,
        ACTIVITY_COMMENT_CREATED,
    };

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

    #[test]
    fn comment_rows_expose_author_details() {
        let row = EventCommentRow {
            id: "comment-1".to_string(),
            event_id: "event-1".to_string(),
            author_sub: "user-1".to_string(),
            author_email: "person@example.com".to_string(),
            author_name: Some("Person".to_string()),
            author_picture_url: Some("https://example.com/person.png".to_string()),
            body: "Looks good.".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let comment = EventComment::from(row);

        assert_eq!(comment.author.sub, "user-1");
        assert_eq!(comment.author.email, "person@example.com");
        assert_eq!(comment.author.name.as_deref(), Some("Person"));
        assert_eq!(
            comment.author.picture_url.as_deref(),
            Some("https://example.com/person.png")
        );
    }

    #[test]
    fn activity_rows_expose_actor_details() {
        let row = EventActivityRow {
            id: "activity-1".to_string(),
            event_id: "event-1".to_string(),
            actor_sub: Some("user-1".to_string()),
            actor_email: Some("person@example.com".to_string()),
            actor_name: Some("Person".to_string()),
            actor_picture_url: None,
            activity_type: ACTIVITY_COMMENT_CREATED.to_string(),
            message: "Person added a comment.".to_string(),
            payload: json!({ "comment_id": "comment-1" }),
            created_at: Utc::now(),
        };

        let activity = EventActivity::from(row);

        let actor = activity.actor.expect("actor details");
        assert_eq!(actor.sub, "user-1");
        assert_eq!(actor.email.as_deref(), Some("person@example.com"));
        assert_eq!(actor.name.as_deref(), Some("Person"));
        assert_eq!(activity.message, "Person added a comment.");
    }
}
