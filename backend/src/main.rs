use std::net::SocketAddr;

use alex_1883_gather_backend::{
    config::BackendConfig, db, email::EmailDispatcher, storage::ObjectStorage,
};
use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    db_pool: PgPool,
    email: EmailDispatcher,
    storage: ObjectStorage,
}

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    database: &'static str,
    email: &'static str,
    object_storage: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "alex_1883_gather_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = BackendConfig::from_env()?;
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port).parse()?;

    let db_pool = db::connect(&config.database).await?;
    db::run_migrations(&db_pool).await?;
    db::verify_connection(&db_pool).await?;
    let email = EmailDispatcher::from_config(&config.email);
    let storage = ObjectStorage::from_config(&config.object_storage);

    let app = app(AppState {
        db_pool,
        email,
        storage,
    });
    let listener = TcpListener::bind(addr).await?;
    tracing::info!(
        %addr,
        self_url = %config.server.self_url,
        email_sender = %config.email.sender_name,
        storage_bucket = %config.object_storage.bucket,
        "backend listening"
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    db::verify_connection(&state.db_pool)
        .await
        .map_err(|error| {
            tracing::error!(%error, "database health check failed");
            StatusCode::SERVICE_UNAVAILABLE
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

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "failed to install terminate signal handler");
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
