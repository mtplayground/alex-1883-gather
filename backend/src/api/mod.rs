pub mod error;
pub mod validation;

use axum::{extract::State, routing::get, Json, Router};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{
    auth::{self, AuthVerifier},
    db,
    email::EmailDispatcher,
    events::{self, EventRepository},
    profile,
    storage::ObjectStorage,
    users::UserRepository,
};

use error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthVerifier,
    pub db_pool: PgPool,
    pub email: EmailDispatcher,
    pub events: EventRepository,
    pub self_url: String,
    pub storage: ObjectStorage,
    pub users: UserRepository,
}

#[derive(serde::Serialize)]
struct ApiInfoResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    database: &'static str,
    email: &'static str,
    object_storage: &'static str,
}

pub fn router(state: AppState) -> Router {
    let auth_state = state.clone();

    Router::new()
        .route("/api", get(api_info))
        .route("/api/auth/login", get(auth::login).post(auth::login_link))
        .route("/api/auth/google", get(auth::google_login))
        .route("/api/auth/google/callback", get(auth::google_callback))
        .route(
            "/api/auth/password-reset/request",
            axum::routing::post(auth::request_password_reset),
        )
        .route(
            "/api/auth/password-reset/complete",
            axum::routing::post(auth::complete_password_reset),
        )
        .route("/api/auth/register", axum::routing::post(auth::register))
        .route("/api/auth/verify", get(auth::verify))
        .route("/api/health", get(health))
        .route("/api/me", get(auth::me))
        .route(
            "/api/events",
            get(events::list_events).post(events::create_event),
        )
        .route(
            "/api/events/:event_id",
            get(events::get_event)
                .put(events::update_event)
                .delete(events::delete_event),
        )
        .route(
            "/api/events/:event_id/cover-image",
            axum::routing::post(events::upload_event_cover_image)
                .put(events::upload_event_cover_image),
        )
        .route(
            "/api/profile",
            get(profile::get_current_profile).put(profile::update_current_profile),
        )
        .route(
            "/api/profile/photo",
            axum::routing::post(profile::upload_profile_photo),
        )
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(axum::middleware::from_fn_with_state(
                    auth_state,
                    auth::session_middleware,
                ))
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(TraceLayer::new_for_http())
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
}

async fn api_info() -> Json<ApiInfoResponse> {
    Json(ApiInfoResponse {
        status: "ok",
        service: "backend",
    })
}

async fn health(State(state): State<AppState>) -> ApiResult<Json<HealthResponse>> {
    db::verify_connection(&state.db_pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "database health check failed");
            ApiError::service_unavailable("database_unavailable", "database health check failed")
        })?;

    Ok(Json(HealthResponse {
        status: "ok",
        service: "backend",
        database: "ok",
        email: if state.email.is_configured() {
            "configured"
        } else {
            "disabled"
        },
        object_storage: if state.storage.bucket().is_empty() {
            "missing"
        } else {
            "configured"
        },
    }))
}

async fn not_found() -> ApiError {
    ApiError::not_found("route_not_found", "route not found")
}
