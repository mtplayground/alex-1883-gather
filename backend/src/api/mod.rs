pub mod error;
pub mod validation;

use axum::{extract::State, routing::get, Json, Router};
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};

use crate::{db, email::EmailDispatcher, storage::ObjectStorage};

use error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: PgPool,
    pub email: EmailDispatcher,
    pub storage: ObjectStorage,
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
    Router::new()
        .route("/api", get(api_info))
        .route("/api/health", get(health))
        .route("/health", get(health))
        .fallback(not_found)
        .with_state(state)
        .layer(
            ServiceBuilder::new()
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
