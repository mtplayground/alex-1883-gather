pub mod error;
pub mod validation;

use axum::{extract::State, http::Uri, response::Html, routing::get, Json, Router};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    trace::TraceLayer,
};

use crate::{
    activity::{self, ActivityRepository},
    auth::{self, AuthVerifier},
    db,
    email::EmailDispatcher,
    events::{self, EventRepository},
    invitations::{self, InvitationRepository},
    profile,
    storage::ObjectStorage,
    users::UserRepository,
};

use error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct AppState {
    pub auth: AuthVerifier,
    pub activity: ActivityRepository,
    pub db_pool: PgPool,
    pub email: EmailDispatcher,
    pub events: EventRepository,
    pub invitations: InvitationRepository,
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
        .route("/api/dashboard/events", get(events::dashboard_events))
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
            "/api/events/:event_id/attachments",
            get(events::list_event_attachments).post(events::upload_event_attachment),
        )
        .route(
            "/api/events/:event_id/attachments/:attachment_id",
            axum::routing::delete(events::delete_event_attachment),
        )
        .route(
            "/api/events/:event_id/attachments/:attachment_id/download",
            get(events::download_event_attachment),
        )
        .route(
            "/api/events/:event_id/comments",
            get(activity::list_event_comments).post(activity::create_event_comment),
        )
        .route(
            "/api/events/:event_id/activity",
            get(activity::list_event_activity),
        )
        .route(
            "/api/events/:event_id/invitations",
            axum::routing::post(invitations::send_event_invitations),
        )
        .route(
            "/api/events/:event_id/attendees",
            get(invitations::list_event_attendees),
        )
        .route(
            "/api/events/:event_id/rsvp",
            axum::routing::put(invitations::update_event_rsvp),
        )
        .route(
            "/api/invitations/:response_token",
            get(invitations::get_invitation_by_token),
        )
        .route(
            "/api/invitations/:response_token/response",
            axum::routing::post(invitations::respond_to_invitation),
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
        .nest_service("/assets", ServeDir::new("frontend/dist/assets"))
        .fallback(static_fallback)
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
        object_storage: if !state.storage.is_configured() {
            "missing"
        } else {
            "configured"
        },
    }))
}

async fn static_fallback(uri: Uri) -> Result<Html<String>, ApiError> {
    if uri.path().starts_with("/api") {
        return Err(not_found().await);
    }

    std::fs::read_to_string("frontend/dist/index.html")
        .map(Html)
        .map_err(|error| {
            tracing::warn!(%error, path = %uri.path(), "frontend asset not found");
            ApiError::not_found("route_not_found", "route not found")
        })
}

async fn not_found() -> ApiError {
    ApiError::not_found("route_not_found", "route not found")
}
