use std::{error::Error, fmt};

use axum::{
    body::Body,
    extract::{Extension, Query, State},
    http::{header, HeaderMap, Request},
    middleware::Next,
    response::{Redirect, Response},
    Json,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

use crate::{
    api::{
        error::{ApiError, ApiResult},
        AppState,
    },
    config::AuthConfig,
    email::{templates, EmailError, EmailMessage},
    users::{RegisteredUser, VerifiedIdentity},
};

#[derive(Clone)]
pub struct AuthVerifier {
    client: reqwest::Client,
    issuer: String,
    audience: String,
    jwks_url: String,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct CurrentUser {
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub registered: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthActionResponse {
    pub user: CurrentUser,
    pub message: String,
    pub email_delivery: EmailDelivery,
}

#[derive(Debug, serde::Serialize)]
pub struct VerificationStatusResponse {
    pub verified: bool,
    pub message: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EmailDelivery {
    pub status: &'static str,
    pub id: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SessionClaims {
    sub: String,
    email: String,
    #[serde(default)]
    email_verified: bool,
    name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Debug, serde::Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    n: String,
    e: String,
}

#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    MissingKeyId,
    SigningKeyNotFound,
    UnsupportedKeyType(String),
    JwksRequest(reqwest::Error),
    Token(jsonwebtoken::errors::Error),
    Url(url::ParseError),
}

impl AuthVerifier {
    pub fn from_config(config: &AuthConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            issuer: config.url.clone(),
            audience: config.app_token.clone(),
            jwks_url: config.jwks_url.clone(),
        }
    }

    pub async fn verify_cookie(&self, headers: &HeaderMap) -> Result<VerifiedIdentity, AuthError> {
        let token = read_cookie(headers, "mctai_session").ok_or(AuthError::MissingToken)?;
        self.verify_token(&token).await
    }

    pub fn login_url(&self, return_to: &str) -> Result<String, AuthError> {
        let mut login_url =
            url::Url::parse(&format!("{}/login", self.issuer.trim_end_matches('/')))
                .map_err(AuthError::Url)?;

        login_url
            .query_pairs_mut()
            .append_pair("app_token", &self.audience)
            .append_pair("return_to", return_to);

        Ok(login_url.to_string())
    }

    async fn verify_token(&self, token: &str) -> Result<VerifiedIdentity, AuthError> {
        let header = decode_header(token).map_err(AuthError::Token)?;
        let kid = header.kid.ok_or(AuthError::MissingKeyId)?;
        let jwk = self.signing_key(&kid).await?;

        if jwk.kty != "RSA" {
            return Err(AuthError::UnsupportedKeyType(jwk.kty));
        }

        let decoding_key =
            DecodingKey::from_rsa_components(&jwk.n, &jwk.e).map_err(AuthError::Token)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_issuer(&[self.issuer.as_str()]);

        let claims = decode::<SessionClaims>(token, &decoding_key, &validation)
            .map_err(AuthError::Token)?
            .claims;

        Ok(VerifiedIdentity {
            sub: claims.sub,
            email: claims.email,
            email_verified: claims.email_verified,
            name: claims.name,
            picture_url: claims.picture,
        })
    }

    async fn signing_key(&self, kid: &str) -> Result<Jwk, AuthError> {
        let jwks = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(AuthError::JwksRequest)?
            .error_for_status()
            .map_err(AuthError::JwksRequest)?
            .json::<JwksResponse>()
            .await
            .map_err(AuthError::JwksRequest)?;

        jwks.keys
            .into_iter()
            .find(|key| key.kid.as_deref() == Some(kid))
            .ok_or(AuthError::SigningKeyNotFound)
    }
}

pub async fn session_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match state.auth.verify_cookie(request.headers()).await {
        Ok(identity) => match state.users.upsert_from_identity(&identity).await {
            Ok(user) => {
                request.extensions_mut().insert(CurrentUser::from(user));
            }
            Err(error) => {
                tracing::error!(%error, "failed to upsert authenticated user");
            }
        },
        Err(AuthError::MissingToken) => {}
        Err(error) => {
            tracing::warn!(%error, "invalid platform auth session");
        }
    }

    next.run(request).await
}

pub async fn me(user: Option<Extension<CurrentUser>>) -> ApiResult<Json<CurrentUser>> {
    let Some(Extension(user)) = user else {
        return Err(ApiError::unauthorized(
            "not_authenticated",
            "valid platform session required",
        ));
    };

    Ok(Json(user))
}

#[derive(Debug, serde::Deserialize)]
pub struct LoginRequest {
    pub return_to: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct LoginResponse {
    pub login_url: String,
}

pub async fn login(
    State(state): State<AppState>,
    Query(query): Query<LoginRequest>,
) -> ApiResult<Redirect> {
    let return_to = frontend_return_to(&state.self_url, query.return_to.as_deref())?;
    let login_url = state
        .auth
        .login_url(&return_to)
        .map_err(ApiError::internal)?;

    Ok(Redirect::to(&login_url))
}

pub async fn login_link(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let return_to = frontend_return_to(&state.self_url, request.return_to.as_deref())?;
    let login_url = state
        .auth
        .login_url(&return_to)
        .map_err(ApiError::internal)?;

    Ok(Json(LoginResponse { login_url }))
}

pub async fn register(
    State(state): State<AppState>,
    user: Option<Extension<CurrentUser>>,
) -> ApiResult<Json<AuthActionResponse>> {
    let Some(Extension(user)) = user else {
        return Err(ApiError::unauthorized(
            "not_authenticated",
            "valid platform session required",
        ));
    };

    let email_delivery = send_registration_email(&state, &user).await;
    let message = if user.registered {
        display_name(&user)
            .map(|name| format!("Registration complete. Welcome in, {name}."))
            .unwrap_or_else(|| "Registration complete. Welcome in.".to_string())
    } else {
        display_name(&user)
            .map(|name| format!("Welcome back, {name}."))
            .unwrap_or_else(|| "Welcome back.".to_string())
    };

    Ok(Json(AuthActionResponse {
        user,
        message,
        email_delivery,
    }))
}

pub async fn verify(
    user: Option<Extension<CurrentUser>>,
) -> ApiResult<Json<VerificationStatusResponse>> {
    let Some(Extension(user)) = user else {
        return Err(ApiError::unauthorized(
            "not_authenticated",
            "valid platform session required",
        ));
    };

    Ok(Json(VerificationStatusResponse {
        verified: user.email_verified,
        message: if user.email_verified {
            "Your email is verified and ready to go.".to_string()
        } else {
            "Your session is valid, but the platform has not marked this email verified yet."
                .to_string()
        },
    }))
}

impl From<RegisteredUser> for CurrentUser {
    fn from(user: RegisteredUser) -> Self {
        Self {
            sub: user.sub,
            email: user.email,
            email_verified: user.email_verified,
            name: user.name,
            picture_url: user.picture_url,
            registered: user.registered,
        }
    }
}

async fn send_registration_email(state: &AppState, user: &CurrentUser) -> EmailDelivery {
    let name = display_name(user).unwrap_or("there");
    let subject = if user.registered {
        "Welcome in - your gathering space is ready"
    } else {
        "Welcome back - your gathering space is ready"
    };
    let html = templates::registration_html(name, user.email_verified);
    let text = if user.email_verified {
        format!("Hi {name}, your account is ready.")
    } else {
        format!(
            "Hi {name}, your account is ready. Your email is still pending platform verification."
        )
    };
    let message = EmailMessage::new(user.email.clone(), subject)
        .html(html)
        .text(text);

    match state.email.send(message).await {
        Ok(Some(dispatch)) => EmailDelivery {
            status: "sent",
            id: Some(dispatch.id),
            message: None,
        },
        Ok(None) => EmailDelivery {
            status: "skipped",
            id: None,
            message: Some("email proxy is not configured".to_string()),
        },
        Err(EmailError::RateLimited) => EmailDelivery {
            status: "rate_limited",
            id: None,
            message: Some("try again shortly".to_string()),
        },
        Err(error) => {
            tracing::error!(%error, "registration email failed");
            EmailDelivery {
                status: "failed",
                id: None,
                message: Some("registration completed, but email could not be sent".to_string()),
            }
        }
    }
}

fn display_name(user: &CurrentUser) -> Option<&str> {
    user.name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| user.email.split('@').next())
}

fn frontend_return_to(self_url: &str, requested: Option<&str>) -> ApiResult<String> {
    let base = url::Url::parse(self_url).map_err(ApiError::internal)?;
    let requested = requested.unwrap_or("/dashboard").trim();
    let return_to = if requested.starts_with('/') {
        base.join(requested).map_err(ApiError::internal)?
    } else {
        url::Url::parse(requested).map_err(|_| {
            ApiError::bad_request("invalid_return_to", "return_to must be a frontend URL")
        })?
    };

    if return_to.origin() != base.origin()
        || return_to.path() == "/api"
        || return_to.path().starts_with("/api/")
    {
        return Err(ApiError::bad_request(
            "invalid_return_to",
            "return_to must point to a user-visible frontend page",
        ));
    }

    Ok(return_to.to_string())
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingToken => write!(formatter, "missing mctai_session cookie"),
            Self::MissingKeyId => write!(formatter, "session token is missing a key id"),
            Self::SigningKeyNotFound => write!(formatter, "matching JWKS signing key not found"),
            Self::UnsupportedKeyType(kty) => write!(formatter, "unsupported JWKS key type {kty}"),
            Self::JwksRequest(error) => write!(formatter, "JWKS request failed: {error}"),
            Self::Token(error) => write!(formatter, "session token verification failed: {error}"),
            Self::Url(error) => write!(formatter, "auth URL construction failed: {error}"),
        }
    }
}

impl Error for AuthError {}

fn read_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(cookie_name, cookie_value)| {
            (cookie_name == name).then(|| cookie_value.to_string())
        })
}

#[cfg(test)]
mod tests {
    use axum::{
        http::{header, HeaderMap, HeaderValue},
        response::IntoResponse,
    };

    use super::{display_name, frontend_return_to, read_cookie, CurrentUser};

    #[test]
    fn reads_named_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            HeaderValue::from_static("theme=sunny; mctai_session=abc.def.ghi"),
        );

        assert_eq!(
            read_cookie(&headers, "mctai_session").as_deref(),
            Some("abc.def.ghi")
        );
    }

    #[test]
    fn display_name_falls_back_to_email_prefix() {
        let user = CurrentUser {
            sub: "user_123".to_string(),
            email: "person@example.com".to_string(),
            email_verified: true,
            name: None,
            picture_url: None,
            registered: true,
        };

        assert_eq!(display_name(&user), Some("person"));
    }

    #[test]
    fn return_to_defaults_to_dashboard() {
        let return_to = frontend_return_to("https://example.test", None).unwrap();

        assert_eq!(return_to, "https://example.test/dashboard");
    }

    #[test]
    fn return_to_rejects_api_paths() {
        let error = frontend_return_to("https://example.test", Some("/api/me")).unwrap_err();

        assert!(error.into_response().status().is_client_error());
    }
}
