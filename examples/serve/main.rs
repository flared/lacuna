//! Start lacuna with a minimal example config.
//!
//! ```sh
//! cargo run --example serve
//! ```

use lacuna::app::AppBuilder;
use lacuna::config::Config;
use lacuna::provider::{Provider, ProviderManager};
use std::path::Path;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("serve")
        .join("serve.config.json");
    let config = Config::load(&config_path)?;

    lacuna::logging::init(&config.lacuna.logging)?;
    lacuna::metrics::init();

    let mut manager = ProviderManager::new();
    for (key, provider_config) in &config.providers {
        let provider = Provider::from_config(key, provider_config)?;
        manager.add(provider);
    }

    let app = AppBuilder::new()
        .manager(manager)
        .identity_header(config.lacuna.identity_header)
        .build();

    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}
