use std::time::Duration;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use crate::{
    api::{
        error::{ApiError, ApiResult},
        validation::{require_max_len, require_non_empty, ValidateRequest, ValidationErrors},
        AppState,
    },
    auth::CurrentUser,
    storage::{safe_key_part, standard_image_type},
};

const EVENT_COVER_IMAGE_MAX_BYTES: usize = 8 * 1024 * 1024;
const EVENT_COVER_IMAGE_URL_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct Event {
    pub id: String,
    pub owner_sub: String,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub timezone: Option<String>,
    pub cover_image_object_key: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventDraft {
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub timezone: Option<String>,
    pub cover_image_object_key: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct EventAttachment {
    pub id: String,
    pub event_id: String,
    pub uploaded_by_sub: String,
    pub object_key: String,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub page_count: Option<i32>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct EventAttachmentDraft {
    pub object_key: String,
    pub filename: String,
    pub content_type: String,
    pub byte_size: i64,
    pub page_count: Option<i32>,
    #[serde(default = "empty_metadata")]
    pub metadata: Value,
}

#[derive(Clone)]
pub struct EventRepository {
    pool: PgPool,
}

#[derive(Debug, serde::Serialize)]
pub struct EventListResponse {
    pub events: Vec<Event>,
}

#[derive(Debug, serde::Serialize)]
pub struct EventCoverImageResponse {
    pub event: Event,
    pub object_key: String,
    pub content_type: String,
    pub access_url: String,
}

impl EventRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, owner_sub: &str, draft: &EventDraft) -> Result<Event, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, Event>(
            r#"
            INSERT INTO events (
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(owner_sub)
        .bind(&draft.title)
        .bind(&draft.description)
        .bind(draft.starts_at)
        .bind(&draft.timezone)
        .bind(&draft.cover_image_object_key)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_for_user(&self, user_sub: &str) -> Result<Vec<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            r#"
            SELECT
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key,
                created_at,
                updated_at
            FROM events
            WHERE
                owner_sub = $1
                OR EXISTS (
                    SELECT 1
                    FROM event_members
                    WHERE
                        event_members.event_id = events.id
                        AND event_members.member_sub = $1
                        AND event_members.status IN ('invited', 'accepted')
                )
            ORDER BY starts_at ASC, created_at ASC
            "#,
        )
        .bind(user_sub)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, event_id: &str) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            r#"
            SELECT
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key,
                created_at,
                updated_at
            FROM events
            WHERE id = $1
            "#,
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn is_invited_member(
        &self,
        event_id: &str,
        user_sub: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM event_members
                WHERE event_id = $1
                    AND member_sub = $2
                    AND status IN ('invited', 'accepted')
            )
            "#,
        )
        .bind(event_id)
        .bind(user_sub)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(
        &self,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            r#"
            UPDATE events
            SET
                title = $2,
                description = $3,
                starts_at = $4,
                timezone = $5,
                cover_image_object_key = $6,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key,
                created_at,
                updated_at
            "#,
        )
        .bind(event_id)
        .bind(&draft.title)
        .bind(&draft.description)
        .bind(draft.starts_at)
        .bind(&draft.timezone)
        .bind(&draft.cover_image_object_key)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_cover_image(
        &self,
        event_id: &str,
        object_key: &str,
    ) -> Result<Option<Event>, sqlx::Error> {
        sqlx::query_as::<_, Event>(
            r#"
            UPDATE events
            SET
                cover_image_object_key = $2,
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id,
                owner_sub,
                title,
                description,
                starts_at,
                timezone,
                cover_image_object_key,
                created_at,
                updated_at
            "#,
        )
        .bind(event_id)
        .bind(object_key)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete(&self, event_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM events WHERE id = $1")
            .bind(event_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}

pub async fn list_events(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
) -> ApiResult<Json<EventListResponse>> {
    let user = require_current_user(user)?;
    let events = state
        .events
        .list_for_user(&user.sub)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventListResponse { events }))
}

pub async fn create_event(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Json(draft): Json<EventDraft>,
) -> ApiResult<(StatusCode, Json<Event>)> {
    let user = require_current_user(user)?;
    draft.validate().map_err(ApiError::validation)?;

    let event = state
        .events
        .create(&user.sub, &draft)
        .await
        .map_err(ApiError::internal)?;

    Ok((StatusCode::CREATED, Json(event)))
}

pub async fn get_event(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<Json<Event>> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;

    ensure_can_read_event(&state.events, &user, &event).await?;

    Ok(Json(event))
}

pub async fn update_event(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    Json(draft): Json<EventDraft>,
) -> ApiResult<Json<Event>> {
    let user = require_current_user(user)?;
    draft.validate().map_err(ApiError::validation)?;

    let existing = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_manage_event(&user, &existing)?;

    let event = state
        .events
        .update(&event_id, &draft)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;

    Ok(Json(event))
}

pub async fn delete_event(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<StatusCode> {
    let user = require_current_user(user)?;
    let existing = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_manage_event(&user, &existing)?;

    state
        .events
        .delete(&event_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn upload_event_cover_image(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<Json<EventCoverImageResponse>> {
    let user = require_current_user(user)?;
    let existing = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_manage_event(&user, &existing)?;

    let upload = read_event_cover_image(&mut multipart).await?;
    let raw_key = event_cover_image_key(&event_id, upload.extension);
    let object_key = state
        .storage
        .put_object(&raw_key, upload.bytes, Some(upload.content_type))
        .await
        .map_err(ApiError::internal)?;

    let event = match state
        .events
        .update_cover_image(&event_id, &object_key)
        .await
    {
        Ok(Some(event)) => event,
        Ok(None) => {
            if let Err(error) = state.storage.delete_object_key(&object_key).await {
                tracing::warn!(%error, object_key, "failed to delete orphaned event cover image");
            }
            return Err(event_not_found());
        }
        Err(error) => {
            if let Err(delete_error) = state.storage.delete_object_key(&object_key).await {
                tracing::warn!(%delete_error, object_key, "failed to delete orphaned event cover image");
            }
            return Err(ApiError::internal(error));
        }
    };

    if let Some(previous_key) = existing.cover_image_object_key {
        if previous_key != object_key {
            if let Err(error) = state.storage.delete_object_key(&previous_key).await {
                tracing::warn!(%error, previous_key, "failed to delete previous event cover image");
            }
        }
    }

    let access_url = state
        .storage
        .presigned_get_url_for_object_key(&object_key, EVENT_COVER_IMAGE_URL_TTL)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventCoverImageResponse {
        event,
        object_key,
        content_type: upload.content_type.to_string(),
        access_url,
    }))
}

impl ValidateRequest for EventDraft {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "title", &self.title);
        require_max_len(&mut errors, "title", &self.title, 160);

        if let Some(description) = &self.description {
            require_max_len(&mut errors, "description", description, 5000);
        }

        if let Some(timezone) = &self.timezone {
            require_max_len(&mut errors, "timezone", timezone, 80);
        }

        if let Some(cover_image_object_key) = &self.cover_image_object_key {
            require_max_len(
                &mut errors,
                "cover_image_object_key",
                cover_image_object_key,
                512,
            );
        }

        errors.into_result()
    }
}

impl ValidateRequest for EventAttachmentDraft {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "object_key", &self.object_key);
        require_max_len(&mut errors, "object_key", &self.object_key, 512);
        require_non_empty(&mut errors, "filename", &self.filename);
        require_max_len(&mut errors, "filename", &self.filename, 255);

        if self.content_type != "application/pdf" {
            errors.push("content_type", "must be application/pdf");
        }

        if self.byte_size < 0 {
            errors.push("byte_size", "must be zero or greater");
        }

        if self.page_count.is_some_and(|page_count| page_count < 0) {
            errors.push("page_count", "must be zero or greater");
        }

        errors.into_result()
    }
}

fn empty_metadata() -> Value {
    Value::Object(Default::default())
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

async fn ensure_can_read_event(
    events: &EventRepository,
    user: &CurrentUser,
    event: &Event,
) -> ApiResult<()> {
    if user_owns_event(user, event) {
        return Ok(());
    }

    if events
        .is_invited_member(&event.id, &user.sub)
        .await
        .map_err(ApiError::internal)?
    {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "event_forbidden",
        "you do not have access to this event",
    ))
}

fn ensure_can_manage_event(user: &CurrentUser, event: &Event) -> ApiResult<()> {
    if user_can_manage_event(user, event) {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "event_forbidden",
        "only the organizer may edit or delete this event",
    ))
}

fn user_owns_event(user: &CurrentUser, event: &Event) -> bool {
    user.sub == event.owner_sub
}

fn user_can_manage_event(user: &CurrentUser, event: &Event) -> bool {
    user_owns_event(user, event)
}

fn event_not_found() -> ApiError {
    ApiError::not_found("event_not_found", "event not found")
}

struct EventCoverImageUpload {
    bytes: Vec<u8>,
    content_type: &'static str,
    extension: &'static str,
}

async fn read_event_cover_image(multipart: &mut Multipart) -> ApiResult<EventCoverImageUpload> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("invalid_upload", "invalid multipart upload"))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name != "cover_image" && field_name != "image" && field_name != "file" {
            continue;
        }

        let content_type = field
            .content_type()
            .and_then(standard_image_type)
            .ok_or_else(|| {
                ApiError::bad_request(
                    "unsupported_image_type",
                    "event cover image must be a JPEG, PNG, WebP, or GIF image",
                )
            })?;
        let bytes = field
            .bytes()
            .await
            .map_err(|_| ApiError::bad_request("invalid_upload", "invalid image upload"))?
            .to_vec();

        if bytes.is_empty() {
            return Err(ApiError::bad_request(
                "empty_upload",
                "event cover image must not be empty",
            ));
        }

        if bytes.len() > EVENT_COVER_IMAGE_MAX_BYTES {
            return Err(ApiError::bad_request(
                "upload_too_large",
                "event cover image must be 8 MB or smaller",
            ));
        }

        return Ok(EventCoverImageUpload {
            bytes,
            content_type: content_type.content_type,
            extension: content_type.extension,
        });
    }

    Err(ApiError::bad_request(
        "missing_cover_image",
        "multipart upload must include a cover image file",
    ))
}

fn event_cover_image_key(event_id: &str, extension: &str) -> String {
    format!(
        "events/{}/cover-{}.{}",
        safe_key_part(event_id),
        chrono::Utc::now().timestamp_millis(),
        extension
    )
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::api::validation::ValidateRequest;
    use crate::auth::CurrentUser;

    use super::{
        event_cover_image_key, user_can_manage_event, user_owns_event, Event, EventAttachmentDraft,
        EventDraft,
    };

    #[test]
    fn event_draft_requires_title() {
        let draft = EventDraft {
            title: " ".to_string(),
            description: None,
            starts_at: Utc::now(),
            timezone: None,
            cover_image_object_key: None,
        };

        assert!(draft.validate().is_err());
    }

    #[test]
    fn attachment_draft_requires_pdf_content_type() {
        let draft = EventAttachmentDraft {
            object_key: "attachments/menu.txt".to_string(),
            filename: "menu.txt".to_string(),
            content_type: "text/plain".to_string(),
            byte_size: 12,
            page_count: Some(1),
            metadata: serde_json::json!({}),
        };

        assert!(draft.validate().is_err());
    }

    #[test]
    fn organizer_can_read_and_manage_event() {
        let user = test_user("organizer-sub");
        let event = test_event("organizer-sub");

        assert!(user_owns_event(&user, &event));
        assert!(user_can_manage_event(&user, &event));
    }

    #[test]
    fn non_organizer_cannot_manage_event() {
        let user = test_user("member-sub");
        let event = test_event("organizer-sub");

        assert!(!user_owns_event(&user, &event));
        assert!(!user_can_manage_event(&user, &event));
    }

    #[test]
    fn cover_image_key_sanitizes_event_id() {
        let key = event_cover_image_key("event/../../1", "png");

        assert!(key.starts_with("events/event_______1/cover-"));
        assert!(key.ends_with(".png"));
    }

    fn test_user(sub: &str) -> CurrentUser {
        CurrentUser {
            sub: sub.to_string(),
            email: format!("{sub}@example.test"),
            email_verified: true,
            name: Some("Test User".to_string()),
            picture_url: None,
            registered: true,
        }
    }

    fn test_event(owner_sub: &str) -> Event {
        let now = Utc::now();

        Event {
            id: "event-1".to_string(),
            owner_sub: owner_sub.to_string(),
            title: "Planning dinner".to_string(),
            description: None,
            starts_at: now,
            timezone: Some("UTC".to_string()),
            cover_image_object_key: None,
            created_at: now,
            updated_at: now,
        }
    }
}
