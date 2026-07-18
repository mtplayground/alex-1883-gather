use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;

use crate::api::validation::{
    require_max_len, require_non_empty, ValidateRequest, ValidationErrors,
};

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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::api::validation::ValidateRequest;

    use super::{EventAttachmentDraft, EventDraft};

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
}
