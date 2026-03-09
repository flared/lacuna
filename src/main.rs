use clap::Parser;
use lacuna::app::AppBuilder;
use lacuna::config::Config;
use lacuna::provider::{Provider, ProviderManager};
use std::path::PathBuf;
use tokio::net::TcpListener;
use tracing::{error, info};

/// An LLM API gateway and proxy
#[derive(Parser)]
struct Args {
    /// Path to the providers config file (YAML or JSON)
    #[arg(long)]
    config: PathBuf,

    /// Path to the assets directory
    #[arg(long, default_value = "frontend/dist")]
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
    let config = Config::load(&args.config).unwrap_or_else(|e| {
        eprintln!("failed to load config: {e}");
        std::process::exit(1);
    });
    info!(count = config.providers.len(), "loaded providers");

    lacuna::logging::init(&config.lacuna.logging).unwrap_or_else(|e| {
        eprintln!("failed to initialize logging: {e}");
        std::process::exit(1);
    });

    lacuna::metrics::init();

    let mut manager = ProviderManager::new();
    for (key, provider_config) in &config.providers {
        let provider = Provider::from_config(key, provider_config).unwrap_or_else(|e| {
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
