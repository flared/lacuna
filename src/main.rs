pub(crate) mod app;
mod auth;
mod config;
mod handlers;
mod logging;
mod metrics;
mod provider;
#[cfg(test)]
mod test_utils;
mod trace;

use app::AppBuilder;
use clap::Parser;
use provider::ProviderManager;
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::{error, info};

#[derive(Parser)]
struct Args {
    /// Path to the providers config file (YAML or JSON)
    #[arg(long)]
    config: PathBuf,

    /// Path to the assets directory
    #[arg(long, default_value = "assets")]
    assets: PathBuf,

    /// Host to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = config::Config::load(&args.config).unwrap_or_else(|e| {
        eprintln!("failed to load config: {e}");
        std::process::exit(1);
    });
    info!(count = config.providers.len(), "loaded providers");

    logging::init(&config.lacuna.logging).unwrap_or_else(|e| {
        eprintln!("failed to initialize logging: {e}");
        std::process::exit(1);
    });

    metrics::init();

    let mut manager = ProviderManager::new();
    for (key, provider_config) in &config.providers {
        let provider = provider::Provider::from_config(key, provider_config).unwrap_or_else(|e| {
            error!(provider = %key, %e, "failed to configure provider");
            std::process::exit(1);
        });
        manager.add(provider);
    }

    let app = AppBuilder::new()
        .manager(manager)
        .assets_path(&args.assets)
        .identity_header(config.lacuna.identity_header)
        .build();

    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port))
        .await
        .unwrap();
    info!(addr = %listener.local_addr().unwrap(), "listening");

    axum::serve(listener, app).await.unwrap();
}
