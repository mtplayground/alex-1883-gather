use std::{error::Error, fmt};

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{header, HeaderMap, Request},
    middleware::Next,
    response::Response,
    Json,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};

use crate::{
    api::{
        error::{ApiError, ApiResult},
        AppState,
    },
    config::AuthConfig,
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

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingToken => write!(formatter, "missing mctai_session cookie"),
            Self::MissingKeyId => write!(formatter, "session token is missing a key id"),
            Self::SigningKeyNotFound => write!(formatter, "matching JWKS signing key not found"),
            Self::UnsupportedKeyType(kty) => write!(formatter, "unsupported JWKS key type {kty}"),
            Self::JwksRequest(error) => write!(formatter, "JWKS request failed: {error}"),
            Self::Token(error) => write!(formatter, "session token verification failed: {error}"),
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
    use axum::http::{header, HeaderMap, HeaderValue};

    use super::read_cookie;

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
}
