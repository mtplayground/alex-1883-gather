use std::time::Duration;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use crate::{
    activity::ACTIVITY_EVENT_EDITED,
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
const EVENT_ATTACHMENT_MAX_BYTES: usize = 25 * 1024 * 1024;
const EVENT_ATTACHMENT_URL_TTL: Duration = Duration::from_secs(60 * 60);
const PDF_CONTENT_TYPE: &str = "application/pdf";

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

#[derive(Debug, serde::Serialize)]
pub struct EventAttachmentListResponse {
    pub attachments: Vec<EventAttachment>,
}

#[derive(Debug, serde::Serialize)]
pub struct EventAttachmentUploadResponse {
    pub attachment: EventAttachment,
    pub access_url: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EventAttachmentDownloadResponse {
    pub attachment: EventAttachment,
    pub access_url: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct DashboardEventsQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct DashboardEventsResponse {
    pub events: Vec<DashboardEventSummary>,
}

#[derive(Debug, serde::Serialize)]
pub struct DashboardEventSummary {
    pub id: String,
    pub owner_sub: String,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub timezone: Option<String>,
    pub cover_image_object_key: Option<String>,
    pub cover_image_url: Option<String>,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct DashboardEventRow {
    pub id: String,
    pub owner_sub: String,
    pub title: String,
    pub description: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub timezone: Option<String>,
    pub cover_image_object_key: Option<String>,
    pub relationship: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

    pub async fn upcoming_for_dashboard(
        &self,
        user_sub: &str,
        limit: i64,
    ) -> Result<Vec<DashboardEventRow>, sqlx::Error> {
        sqlx::query_as::<_, DashboardEventRow>(
            r#"
            SELECT
                events.id,
                events.owner_sub,
                events.title,
                events.description,
                events.starts_at,
                events.timezone,
                events.cover_image_object_key,
                CASE
                    WHEN events.owner_sub = $1 THEN 'organizer'
                    WHEN EXISTS (
                        SELECT 1
                        FROM event_members
                        WHERE
                            event_members.event_id = events.id
                            AND event_members.member_sub = $1
                            AND event_members.status = 'accepted'
                    ) THEN 'joined'
                    ELSE 'invited'
                END AS relationship,
                events.created_at,
                events.updated_at
            FROM events
            WHERE
                events.starts_at >= NOW()
                AND (
                    events.owner_sub = $1
                    OR EXISTS (
                        SELECT 1
                        FROM event_members
                        WHERE
                            event_members.event_id = events.id
                            AND event_members.member_sub = $1
                            AND event_members.status IN ('invited', 'accepted')
                    )
                )
            ORDER BY events.starts_at ASC, events.created_at ASC
            LIMIT $2
            "#,
        )
        .bind(user_sub)
        .bind(limit)
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

    pub async fn create_attachment(
        &self,
        event_id: &str,
        uploaded_by_sub: &str,
        draft: &EventAttachmentDraft,
    ) -> Result<EventAttachment, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query_as::<_, EventAttachment>(
            r#"
            INSERT INTO event_attachments (
                id,
                event_id,
                uploaded_by_sub,
                object_key,
                filename,
                content_type,
                byte_size,
                page_count,
                metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING
                id,
                event_id,
                uploaded_by_sub,
                object_key,
                filename,
                content_type,
                byte_size,
                page_count,
                metadata,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(event_id)
        .bind(uploaded_by_sub)
        .bind(&draft.object_key)
        .bind(&draft.filename)
        .bind(&draft.content_type)
        .bind(draft.byte_size)
        .bind(draft.page_count)
        .bind(draft.metadata.clone())
        .fetch_one(&self.pool)
        .await
    }

    pub async fn list_attachments(
        &self,
        event_id: &str,
    ) -> Result<Vec<EventAttachment>, sqlx::Error> {
        sqlx::query_as::<_, EventAttachment>(
            r#"
            SELECT
                id,
                event_id,
                uploaded_by_sub,
                object_key,
                filename,
                content_type,
                byte_size,
                page_count,
                metadata,
                created_at,
                updated_at
            FROM event_attachments
            WHERE event_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_attachment(
        &self,
        event_id: &str,
        attachment_id: &str,
    ) -> Result<Option<EventAttachment>, sqlx::Error> {
        sqlx::query_as::<_, EventAttachment>(
            r#"
            SELECT
                id,
                event_id,
                uploaded_by_sub,
                object_key,
                filename,
                content_type,
                byte_size,
                page_count,
                metadata,
                created_at,
                updated_at
            FROM event_attachments
            WHERE event_id = $1 AND id = $2
            "#,
        )
        .bind(event_id)
        .bind(attachment_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn delete_attachment(
        &self,
        event_id: &str,
        attachment_id: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM event_attachments WHERE event_id = $1 AND id = $2")
            .bind(event_id)
            .bind(attachment_id)
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

pub async fn dashboard_events(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Query(query): Query<DashboardEventsQuery>,
) -> ApiResult<Json<DashboardEventsResponse>> {
    let user = require_current_user(user)?;
    let events = state
        .events
        .upcoming_for_dashboard(&user.sub, dashboard_event_limit(query.limit))
        .await
        .map_err(ApiError::internal)?;
    let mut summaries = Vec::with_capacity(events.len());

    for event in events {
        let cover_image_url = match &event.cover_image_object_key {
            Some(object_key) => Some(
                state
                    .storage
                    .presigned_get_url_for_object_key(object_key, EVENT_COVER_IMAGE_URL_TTL)
                    .await
                    .map_err(ApiError::internal)?,
            ),
            None => None,
        };

        summaries.push(DashboardEventSummary {
            id: event.id,
            owner_sub: event.owner_sub,
            title: event.title,
            description: event.description,
            starts_at: event.starts_at,
            timezone: event.timezone,
            cover_image_object_key: event.cover_image_object_key,
            cover_image_url,
            relationship: event.relationship,
            created_at: event.created_at,
            updated_at: event.updated_at,
        });
    }

    Ok(Json(DashboardEventsResponse { events: summaries }))
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

    let changed_fields = changed_event_fields(&existing, &draft);
    let event = state
        .events
        .update(&event_id, &draft)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;

    if !changed_fields.is_empty() {
        let actor = display_name(&user);
        let message = format!("{actor} updated the event details.");

        state
            .activity
            .record_activity(
                &event.id,
                Some(&user.sub),
                ACTIVITY_EVENT_EDITED,
                &message,
                serde_json::json!({
                    "changed_fields": changed_fields,
                }),
            )
            .await
            .map_err(ApiError::internal)?;
    }

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

pub async fn list_event_attachments(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
) -> ApiResult<Json<EventAttachmentListResponse>> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_read_event(&state.events, &user, &event).await?;

    let attachments = state
        .events
        .list_attachments(&event_id)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventAttachmentListResponse { attachments }))
}

pub async fn upload_event_attachment(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path(event_id): Path<String>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<EventAttachmentUploadResponse>)> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_read_event(&state.events, &user, &event).await?;

    let upload = read_event_attachment(&event_id, &mut multipart).await?;
    let object_key = state
        .storage
        .put_object(&upload.raw_key, upload.bytes, Some(PDF_CONTENT_TYPE))
        .await
        .map_err(ApiError::internal)?;
    let draft = EventAttachmentDraft {
        object_key: object_key.clone(),
        filename: upload.filename,
        content_type: PDF_CONTENT_TYPE.to_string(),
        byte_size: upload.byte_size,
        page_count: None,
        metadata: serde_json::json!({}),
    };
    draft.validate().map_err(ApiError::validation)?;

    let attachment = match state
        .events
        .create_attachment(&event_id, &user.sub, &draft)
        .await
    {
        Ok(attachment) => attachment,
        Err(error) => {
            if let Err(delete_error) = state.storage.delete_object_key(&object_key).await {
                tracing::warn!(%delete_error, object_key, "failed to delete orphaned event attachment");
            }
            return Err(ApiError::internal(error));
        }
    };
    let access_url = state
        .storage
        .presigned_get_url_for_object_key(&attachment.object_key, EVENT_ATTACHMENT_URL_TTL)
        .await
        .map_err(ApiError::internal)?;

    Ok((
        StatusCode::CREATED,
        Json(EventAttachmentUploadResponse {
            attachment,
            access_url,
        }),
    ))
}

pub async fn download_event_attachment(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path((event_id, attachment_id)): Path<(String, String)>,
) -> ApiResult<Json<EventAttachmentDownloadResponse>> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    ensure_can_read_event(&state.events, &user, &event).await?;

    let attachment = state
        .events
        .get_attachment(&event_id, &attachment_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(attachment_not_found)?;
    let access_url = state
        .storage
        .presigned_get_url_for_object_key(&attachment.object_key, EVENT_ATTACHMENT_URL_TTL)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(EventAttachmentDownloadResponse {
        attachment,
        access_url,
    }))
}

pub async fn delete_event_attachment(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Path((event_id, attachment_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let user = require_current_user(user)?;
    let event = state
        .events
        .get(&event_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(event_not_found)?;
    let attachment = state
        .events
        .get_attachment(&event_id, &attachment_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(attachment_not_found)?;
    ensure_can_remove_attachment(&user, &event, &attachment)?;

    state
        .events
        .delete_attachment(&event_id, &attachment_id)
        .await
        .map_err(ApiError::internal)?;

    if let Err(error) = state
        .storage
        .delete_object_key(&attachment.object_key)
        .await
    {
        tracing::warn!(
            %error,
            object_key = attachment.object_key,
            "failed to delete event attachment object"
        );
    }

    Ok(StatusCode::NO_CONTENT)
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

fn changed_event_fields(existing: &Event, draft: &EventDraft) -> Vec<&'static str> {
    let mut fields = Vec::new();

    if existing.title != draft.title.trim() {
        fields.push("title");
    }

    if existing.description.as_deref().unwrap_or("").trim()
        != draft.description.as_deref().unwrap_or("").trim()
    {
        fields.push("description");
    }

    if existing.starts_at != draft.starts_at {
        fields.push("starts_at");
    }

    if existing.timezone.as_deref().unwrap_or("").trim()
        != draft.timezone.as_deref().unwrap_or("").trim()
    {
        fields.push("timezone");
    }

    if existing
        .cover_image_object_key
        .as_deref()
        .unwrap_or("")
        .trim()
        != draft.cover_image_object_key.as_deref().unwrap_or("").trim()
    {
        fields.push("cover_image_object_key");
    }

    fields
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

fn display_name(user: &CurrentUser) -> String {
    user.name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| user.email.split('@').next())
        .unwrap_or("The organizer")
        .to_string()
}

fn ensure_can_remove_attachment(
    user: &CurrentUser,
    event: &Event,
    attachment: &EventAttachment,
) -> ApiResult<()> {
    if user_can_manage_event(user, event) || attachment.uploaded_by_sub == user.sub {
        return Ok(());
    }

    Err(ApiError::forbidden(
        "attachment_forbidden",
        "only the organizer or uploader may remove this attachment",
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

fn dashboard_event_limit(limit: Option<i64>) -> i64 {
    limit.unwrap_or(25).clamp(1, 100)
}

fn attachment_not_found() -> ApiError {
    ApiError::not_found("attachment_not_found", "attachment not found")
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

struct EventAttachmentUpload {
    bytes: Vec<u8>,
    raw_key: String,
    filename: String,
    byte_size: i64,
}

async fn read_event_attachment(
    event_id: &str,
    multipart: &mut Multipart,
) -> ApiResult<EventAttachmentUpload> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("invalid_upload", "invalid multipart upload"))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name != "attachment" && field_name != "file" {
            continue;
        }

        if field.content_type() != Some(PDF_CONTENT_TYPE) {
            return Err(ApiError::bad_request(
                "unsupported_attachment_type",
                "event attachment must be a PDF file",
            ));
        }

        let filename = normalize_pdf_filename(field.file_name());
        let bytes = field
            .bytes()
            .await
            .map_err(|_| ApiError::bad_request("invalid_upload", "invalid PDF upload"))?
            .to_vec();

        if bytes.is_empty() {
            return Err(ApiError::bad_request(
                "empty_upload",
                "event attachment must not be empty",
            ));
        }

        if bytes.len() > EVENT_ATTACHMENT_MAX_BYTES {
            return Err(ApiError::bad_request(
                "upload_too_large",
                "event attachment must be 25 MB or smaller",
            ));
        }

        if !looks_like_pdf(&bytes) {
            return Err(ApiError::bad_request(
                "invalid_pdf",
                "event attachment must be a valid PDF file",
            ));
        }

        let raw_key = event_attachment_key(event_id, &filename);

        return Ok(EventAttachmentUpload {
            byte_size: bytes.len() as i64,
            bytes,
            raw_key,
            filename,
        });
    }

    Err(ApiError::bad_request(
        "missing_attachment",
        "multipart upload must include a PDF attachment file",
    ))
}

fn event_attachment_key(event_id: &str, filename: &str) -> String {
    format!(
        "events/{}/attachments/{}-{}",
        safe_key_part(event_id),
        uuid::Uuid::new_v4(),
        safe_key_part(filename)
    )
}

fn normalize_pdf_filename(file_name: Option<&str>) -> String {
    let name = file_name
        .and_then(|file_name| file_name.rsplit(['/', '\\']).next())
        .unwrap_or_default()
        .trim();
    let name = if name.is_empty() {
        "attachment.pdf".to_string()
    } else {
        name.to_string()
    };
    let name = if name.to_ascii_lowercase().ends_with(".pdf") {
        name
    } else {
        format!("{name}.pdf")
    };

    if name.len() <= 255 {
        return name;
    }

    let stem: String = name.chars().take(251).collect();
    format!("{stem}.pdf")
}

fn looks_like_pdf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"%PDF-")
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::api::validation::ValidateRequest;
    use crate::auth::CurrentUser;

    use super::{
        dashboard_event_limit, event_attachment_key, event_cover_image_key, looks_like_pdf,
        normalize_pdf_filename, user_can_manage_event, user_owns_event, Event, EventAttachment,
        EventAttachmentDraft, EventDraft, PDF_CONTENT_TYPE,
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

    #[test]
    fn attachment_key_sanitizes_event_id_and_filename() {
        let key = event_attachment_key("event/../../1", "../timeline.pdf");

        assert!(key.starts_with("events/event_______1/attachments/"));
        assert!(key.ends_with("-___timeline_pdf"));
    }

    #[test]
    fn pdf_filename_normalization_is_stable() {
        assert_eq!(
            normalize_pdf_filename(Some("../Floor Plan")),
            "Floor Plan.pdf"
        );
        assert_eq!(normalize_pdf_filename(None), "attachment.pdf");
    }

    #[test]
    fn pdf_header_validation_checks_magic_bytes() {
        assert!(looks_like_pdf(b"%PDF-1.7\n"));
        assert!(!looks_like_pdf(b"not a pdf"));
    }

    #[test]
    fn dashboard_limit_uses_safe_bounds() {
        assert_eq!(dashboard_event_limit(None), 25);
        assert_eq!(dashboard_event_limit(Some(0)), 1);
        assert_eq!(dashboard_event_limit(Some(250)), 100);
    }

    #[test]
    fn uploader_can_remove_own_attachment() {
        let user = test_user("member-sub");
        let event = test_event("organizer-sub");
        let attachment = test_attachment("member-sub");

        assert!(super::ensure_can_remove_attachment(&user, &event, &attachment).is_ok());
    }

    #[test]
    fn unrelated_user_cannot_remove_attachment() {
        let user = test_user("other-sub");
        let event = test_event("organizer-sub");
        let attachment = test_attachment("member-sub");

        assert!(super::ensure_can_remove_attachment(&user, &event, &attachment).is_err());
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

    fn test_attachment(uploaded_by_sub: &str) -> EventAttachment {
        let now = Utc::now();

        EventAttachment {
            id: "attachment-1".to_string(),
            event_id: "event-1".to_string(),
            uploaded_by_sub: uploaded_by_sub.to_string(),
            object_key: "events/event-1/attachments/attachment-1-menu_pdf".to_string(),
            filename: "menu.pdf".to_string(),
            content_type: PDF_CONTENT_TYPE.to_string(),
            byte_size: 1024,
            page_count: None,
            metadata: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }
}
