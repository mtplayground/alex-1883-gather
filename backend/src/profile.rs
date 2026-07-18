use axum::{
    extract::{Extension, State},
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
