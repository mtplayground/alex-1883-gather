use std::time::Duration;

use axum::{
    extract::{Extension, Multipart, State},
    Json,
};

use crate::{
    api::{
        error::{ApiError, ApiResult},
        validation::ValidateRequest,
        AppState,
    },
    auth::CurrentUser,
    users::{Profile, ProfileUpdate, User, UserRepository},
};

const PROFILE_PHOTO_MAX_BYTES: usize = 5 * 1024 * 1024;
const PROFILE_PHOTO_URL_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, serde::Serialize)]
pub struct ProfileResponse {
    pub account: AccountSettings,
    pub profile: Profile,
}

#[derive(Debug, serde::Serialize)]
pub struct AccountSettings {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProfilePhotoResponse {
    pub profile: Profile,
    pub object_key: String,
    pub content_type: String,
    pub access_url: String,
}

pub async fn get_current_profile(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
) -> ApiResult<Json<ProfileResponse>> {
    let user = require_current_user(user)?;
    let service = ProfileService::new(&state.users);
    let response = service.load(&user.sub).await?;

    Ok(Json(response))
}

pub async fn update_current_profile(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    Json(update): Json<ProfileUpdate>,
) -> ApiResult<Json<ProfileResponse>> {
    let user = require_current_user(user)?;
    update.validate().map_err(ApiError::validation)?;

    let service = ProfileService::new(&state.users);
    let response = service.update(&user.sub, &update).await?;

    Ok(Json(response))
}

pub async fn upload_profile_photo(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
    mut multipart: Multipart,
) -> ApiResult<Json<ProfilePhotoResponse>> {
    let user = require_current_user(user)?;
    let upload = read_profile_photo(&mut multipart).await?;
    let service = ProfileService::new(&state.users);
    let existing_profile = service.load_profile(&user.sub).await?;
    let raw_key = profile_photo_key(&user.sub, upload.extension);
    let object_key = state
        .storage
        .put_object(&raw_key, upload.bytes, Some(upload.content_type))
        .await
        .map_err(ApiError::internal)?;
    let profile = service.update_photo(&user.sub, &object_key).await?;

    if let Some(previous_key) = existing_profile.photo_object_key {
        if previous_key != object_key {
            if let Err(error) = state.storage.delete_object_key(&previous_key).await {
                tracing::warn!(%error, previous_key, "failed to delete previous profile photo");
            }
        }
    }

    let access_url = state
        .storage
        .presigned_get_url_for_object_key(&object_key, PROFILE_PHOTO_URL_TTL)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(ProfilePhotoResponse {
        profile,
        object_key,
        content_type: upload.content_type.to_string(),
        access_url,
    }))
}

struct ProfileService<'a> {
    users: &'a UserRepository,
}

impl<'a> ProfileService<'a> {
    fn new(users: &'a UserRepository) -> Self {
        Self { users }
    }

    async fn load(&self, sub: &str) -> ApiResult<ProfileResponse> {
        let account = self.load_account(sub).await?;
        let profile = self.load_profile(sub).await?;

        Ok(ProfileResponse { account, profile })
    }

    async fn update_photo(&self, sub: &str, object_key: &str) -> ApiResult<Profile> {
        self.users
            .update_profile_photo(sub, object_key)
            .await
            .map_err(map_profile_write_error)
    }

    async fn update(&self, sub: &str, update: &ProfileUpdate) -> ApiResult<ProfileResponse> {
        let profile = self
            .users
            .update_profile(sub, update)
            .await
            .map_err(map_profile_write_error)?;
        let account = self.load_account(sub).await?;

        Ok(ProfileResponse { account, profile })
    }

    async fn load_account(&self, sub: &str) -> ApiResult<AccountSettings> {
        self.users
            .get_user(sub)
            .await
            .map_err(ApiError::internal)?
            .map(AccountSettings::from)
            .ok_or_else(|| ApiError::not_found("account_not_found", "account not found"))
    }

    async fn load_profile(&self, sub: &str) -> ApiResult<Profile> {
        self.users
            .get_profile(sub)
            .await
            .map_err(ApiError::internal)?
            .ok_or_else(|| ApiError::not_found("profile_not_found", "profile not found"))
    }
}

impl From<User> for AccountSettings {
    fn from(user: User) -> Self {
        Self {
            sub: user.sub,
            email: user.email,
            email_verified: user.email_verified,
            name: user.name,
            picture_url: user.picture_url,
            created_at: user.created_at,
            last_seen_at: user.last_seen_at,
        }
    }
}

fn require_current_user(user: Option<Extension<CurrentUser>>) -> ApiResult<CurrentUser> {
    user.map(|Extension(user)| user).ok_or_else(|| {
        ApiError::unauthorized("not_authenticated", "valid platform session required")
    })
}

fn map_profile_write_error(error: sqlx::Error) -> ApiError {
    match error {
        sqlx::Error::RowNotFound => ApiError::not_found("profile_not_found", "profile not found"),
        error => ApiError::internal(error),
    }
}

struct ProfilePhotoUpload {
    bytes: Vec<u8>,
    content_type: &'static str,
    extension: &'static str,
}

async fn read_profile_photo(multipart: &mut Multipart) -> ApiResult<ProfilePhotoUpload> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiError::bad_request("invalid_upload", "invalid multipart upload"))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name != "photo" && field_name != "file" {
            continue;
        }

        let content_type = field.content_type().and_then(image_type).ok_or_else(|| {
            ApiError::bad_request(
                "unsupported_image_type",
                "profile photo must be a JPEG, PNG, WebP, or GIF image",
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
                "profile photo must not be empty",
            ));
        }

        if bytes.len() > PROFILE_PHOTO_MAX_BYTES {
            return Err(ApiError::bad_request(
                "upload_too_large",
                "profile photo must be 5 MB or smaller",
            ));
        }

        return Ok(ProfilePhotoUpload {
            bytes,
            content_type: content_type.0,
            extension: content_type.1,
        });
    }

    Err(ApiError::bad_request(
        "missing_photo",
        "multipart upload must include a photo file",
    ))
}

fn image_type(content_type: &str) -> Option<(&'static str, &'static str)> {
    match content_type {
        "image/jpeg" | "image/jpg" => Some(("image/jpeg", "jpg")),
        "image/png" => Some(("image/png", "png")),
        "image/webp" => Some(("image/webp", "webp")),
        "image/gif" => Some(("image/gif", "gif")),
        _ => None,
    }
}

fn profile_photo_key(sub: &str, extension: &str) -> String {
    format!(
        "profiles/{}/photo-{}.{}",
        safe_key_part(sub),
        chrono::Utc::now().timestamp_millis(),
        extension
    )
}

fn safe_key_part(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{image_type, safe_key_part};

    #[test]
    fn image_type_accepts_standard_images() {
        assert_eq!(image_type("image/jpeg"), Some(("image/jpeg", "jpg")));
        assert_eq!(image_type("image/png"), Some(("image/png", "png")));
        assert_eq!(image_type("image/webp"), Some(("image/webp", "webp")));
        assert_eq!(image_type("image/gif"), Some(("image/gif", "gif")));
    }

    #[test]
    fn safe_key_part_replaces_path_separators() {
        assert_eq!(safe_key_part("user|abc/123"), "user_abc_123");
    }
}
