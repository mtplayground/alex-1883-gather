use std::net::SocketAddr;

use alex_1883_gather_backend::{
    activity::ActivityRepository,
    api::{self, AppState},
    auth::AuthVerifier,
    config::BackendConfig,
    db,
    email::EmailDispatcher,
    events::EventRepository,
    invitations::InvitationRepository,
    reminders,
    storage::ObjectStorage,
    users::UserRepository,
};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    let activity = ActivityRepository::new(db_pool.clone());
    let events = EventRepository::new(db_pool.clone());
    let invitations = InvitationRepository::new(db_pool.clone());
    let storage = ObjectStorage::from_config(&config.object_storage);
    let users = UserRepository::new(db_pool.clone());
    let auth = AuthVerifier::from_config(&config.auth);
    reminders::spawn_scheduler(
        db_pool.clone(),
        email.clone(),
        config.server.self_url.clone(),
    );

    let state = AppState {
        auth,
        activity,
        db_pool,
        email,
        events,
        invitations,
        self_url: config.server.self_url.clone(),
        storage,
        users,
    };
    let app = api::router(state);
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
