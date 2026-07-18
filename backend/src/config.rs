use std::{env, error::Error, fmt};

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub object_storage: ObjectStorageConfig,
    pub email: EmailConfig,
    pub auth: AuthConfig,
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub self_url: String,
}

#[derive(Clone, Debug)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Clone, Debug)]
pub struct ObjectStorageConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub prefix: String,
}

#[derive(Clone, Debug)]
pub struct EmailConfig {
    pub url: Option<String>,
    pub app_token: Option<String>,
    pub sender_name: String,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    pub url: String,
    pub app_token: String,
    pub jwks_url: String,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingVar(&'static str),
    EmptyVar(&'static str),
    InvalidPort { name: &'static str, value: String },
    InvalidUrl { name: &'static str, value: String },
}

impl BackendConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            server: ServerConfig::from_env()?,
            database: DatabaseConfig {
                url: required_var("DATABASE_URL")?,
            },
            object_storage: ObjectStorageConfig {
                endpoint: required_url("OBJECT_STORAGE_ENDPOINT")?,
                region: required_var("OBJECT_STORAGE_REGION")?,
                bucket: required_var("OBJECT_STORAGE_BUCKET")?,
                access_key_id: required_var("OBJECT_STORAGE_ACCESS_KEY_ID")?,
                secret_access_key: required_var("OBJECT_STORAGE_SECRET_ACCESS_KEY")?,
                prefix: env::var("OBJECT_STORAGE_PREFIX")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "app".to_string()),
            },
            email: EmailConfig {
                url: optional_url("MCTAI_EMAIL_URL")?,
                app_token: optional_var("MCTAI_EMAIL_APP_TOKEN"),
                sender_name: env::var("MCTAI_EMAIL_SENDER_NAME")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "App Notifications".to_string()),
            },
            auth: AuthConfig {
                url: required_url("MCTAI_AUTH_URL")?,
                app_token: required_var("MCTAI_AUTH_APP_TOKEN")?,
                jwks_url: required_url("MCTAI_AUTH_JWKS_URL")?,
            },
        })
    }
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: env::var("HOST")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "0.0.0.0".to_string()),
            port: optional_port("PORT", 8080)?,
            self_url: required_url("SELF_URL")?,
        })
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVar(name) => {
                write!(formatter, "missing required environment variable {name}")
            }
            Self::EmptyVar(name) => {
                write!(formatter, "environment variable {name} must not be empty")
            }
            Self::InvalidPort { name, value } => {
                write!(
                    formatter,
                    "environment variable {name} is not a valid port: {value}"
                )
            }
            Self::InvalidUrl { name, value } => {
                write!(
                    formatter,
                    "environment variable {name} must be an http(s) URL: {value}"
                )
            }
        }
    }
}

impl Error for ConfigError {}

fn required_var(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Err(ConfigError::EmptyVar(name)),
        Ok(value) => Ok(value),
        Err(_) => Err(ConfigError::MissingVar(name)),
    }
}

fn required_url(name: &'static str) -> Result<String, ConfigError> {
    let value = required_var(name)?;
    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(value)
    } else {
        Err(ConfigError::InvalidUrl { name, value })
    }
}

fn optional_var(name: &'static str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn optional_url(name: &'static str) -> Result<Option<String>, ConfigError> {
    let Some(value) = optional_var(name) else {
        return Ok(None);
    };

    if value.starts_with("https://") || value.starts_with("http://") {
        Ok(Some(value))
    } else {
        Err(ConfigError::InvalidUrl { name, value })
    }
}

fn optional_port(name: &'static str, default: u16) -> Result<u16, ConfigError> {
    match env::var(name) {
        Ok(value) if value.trim().is_empty() => Ok(default),
        Ok(value) => value
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort { name, value }),
        Err(_) => Ok(default),
    }
}
