use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::api::validation::{
    require_max_len, require_non_empty, ValidateRequest, ValidationErrors,
};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct VerifiedIdentity {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture_url: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct User {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct UserIdentity {
    pub provider: String,
    pub provider_subject: String,
    pub user_sub: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct Profile {
    pub user_sub: String,
    pub display_name: String,
    pub photo_object_key: Option<String>,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, FromRow)]
pub struct RegisteredUser {
    pub registered: bool,
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProfileUpdate {
    pub display_name: String,
    pub photo_object_key: Option<String>,
    pub bio: Option<String>,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn upsert_from_identity(
        &self,
        identity: &VerifiedIdentity,
    ) -> Result<RegisteredUser, sqlx::Error> {
        let registered_user = sqlx::query_as::<_, RegisteredUser>(
            r#"
            INSERT INTO users (sub, email, email_verified, name, picture_url, last_seen_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (sub) DO UPDATE SET
                email = EXCLUDED.email,
                email_verified = EXCLUDED.email_verified,
                name = EXCLUDED.name,
                picture_url = EXCLUDED.picture_url,
                last_seen_at = NOW()
            RETURNING
                (xmax = 0) AS registered,
                sub,
                email,
                email_verified,
                name,
                picture_url,
                created_at,
                last_seen_at
            "#,
        )
        .bind(&identity.sub)
        .bind(&identity.email)
        .bind(identity.email_verified)
        .bind(&identity.name)
        .bind(&identity.picture_url)
        .fetch_one(&self.pool)
        .await?;

        self.upsert_identity_link(identity).await?;
        self.ensure_profile(identity).await?;

        Ok(registered_user)
    }

    pub async fn get_user(&self, sub: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT sub, email, email_verified, name, picture_url, created_at, last_seen_at
            FROM users
            WHERE sub = $1
            "#,
        )
        .bind(sub)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn get_profile(&self, sub: &str) -> Result<Option<Profile>, sqlx::Error> {
        sqlx::query_as::<_, Profile>(
            r#"
            SELECT user_sub, display_name, photo_object_key, bio, created_at, updated_at
            FROM profiles
            WHERE user_sub = $1
            "#,
        )
        .bind(sub)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn update_profile(
        &self,
        sub: &str,
        update: &ProfileUpdate,
    ) -> Result<Profile, sqlx::Error> {
        sqlx::query_as::<_, Profile>(
            r#"
            UPDATE profiles
            SET display_name = $2,
                photo_object_key = $3,
                bio = $4,
                updated_at = NOW()
            WHERE user_sub = $1
            RETURNING user_sub, display_name, photo_object_key, bio, created_at, updated_at
            "#,
        )
        .bind(sub)
        .bind(&update.display_name)
        .bind(&update.photo_object_key)
        .bind(&update.bio)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_profile_photo(
        &self,
        sub: &str,
        photo_object_key: &str,
    ) -> Result<Profile, sqlx::Error> {
        sqlx::query_as::<_, Profile>(
            r#"
            UPDATE profiles
            SET photo_object_key = $2,
                updated_at = NOW()
            WHERE user_sub = $1
            RETURNING user_sub, display_name, photo_object_key, bio, created_at, updated_at
            "#,
        )
        .bind(sub)
        .bind(photo_object_key)
        .fetch_one(&self.pool)
        .await
    }

    async fn upsert_identity_link(&self, identity: &VerifiedIdentity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO user_identities (provider, provider_subject, user_sub, email, last_seen_at)
            VALUES ('mctai', $1, $1, $2, NOW())
            ON CONFLICT (provider, provider_subject) DO UPDATE SET
                email = EXCLUDED.email,
                last_seen_at = NOW()
            "#,
        )
        .bind(&identity.sub)
        .bind(&identity.email)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    async fn ensure_profile(&self, identity: &VerifiedIdentity) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO profiles (user_sub, display_name, photo_object_key)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_sub) DO NOTHING
            "#,
        )
        .bind(&identity.sub)
        .bind(default_display_name(identity))
        .bind(Option::<String>::None)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }
}

impl ValidateRequest for ProfileUpdate {
    fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::new();

        require_non_empty(&mut errors, "display_name", &self.display_name);
        require_max_len(&mut errors, "display_name", &self.display_name, 80);

        if let Some(bio) = &self.bio {
            require_max_len(&mut errors, "bio", bio, 280);
        }

        if let Some(photo_object_key) = &self.photo_object_key {
            require_max_len(&mut errors, "photo_object_key", photo_object_key, 512);
        }

        errors.into_result()
    }
}

fn default_display_name(identity: &VerifiedIdentity) -> String {
    identity
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| identity.email.split('@').next())
        .unwrap_or("Guest")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::api::validation::ValidateRequest;

    use super::{default_display_name, ProfileUpdate, VerifiedIdentity};

    #[test]
    fn default_display_name_prefers_identity_name() {
        let identity = VerifiedIdentity {
            sub: "user_123".to_string(),
            email: "person@example.com".to_string(),
            email_verified: true,
            name: Some("Alex".to_string()),
            picture_url: None,
        };

        assert_eq!(default_display_name(&identity), "Alex");
    }

    #[test]
    fn default_display_name_falls_back_to_email_prefix() {
        let identity = VerifiedIdentity {
            sub: "user_123".to_string(),
            email: "person@example.com".to_string(),
            email_verified: true,
            name: None,
            picture_url: None,
        };

        assert_eq!(default_display_name(&identity), "person");
    }

    #[test]
    fn profile_update_requires_display_name() {
        let update = ProfileUpdate {
            display_name: " ".to_string(),
            photo_object_key: None,
            bio: None,
        };

        assert!(update.validate().is_err());
    }
}
