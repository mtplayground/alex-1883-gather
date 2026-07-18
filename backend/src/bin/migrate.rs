use std::{env, io};

use alex_1883_gather_backend::{config::DatabaseConfig, db};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL").map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "missing required environment variable DATABASE_URL",
        )
    })?;
    let config = DatabaseConfig { url: database_url };
    let db_pool = db::connect(&config).await?;
    db::run_migrations(&db_pool).await?;

    tracing::info!("database migrations complete");
    Ok(())
}
