use alex_1883_gather_backend::config::BackendConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = BackendConfig::from_env()?;

    println!(
        "config ok: {}:{} -> {}",
        config.server.host, config.server.port, config.server.self_url
    );
    println!(
        "database url present; object storage bucket: {}; email proxy: {}",
        config.object_storage.bucket,
        if config.email.url.is_some() && config.email.app_token.is_some() {
            "configured"
        } else {
            "not configured"
        }
    );

    Ok(())
}
