mod config;
mod handlers;
mod logging;
mod provider;

use axum::{Router, routing::get};
use clap::Parser;
use provider::ProviderManager;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[derive(Parser)]
struct Args {
    /// Path to the providers config file (YAML or JSON)
    #[arg(long)]
    config: PathBuf,

    /// Host to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

pub(crate) fn app(manager: ProviderManager) -> Router {
    let mut router = Router::new().route("/health", get(handlers::health::health));

    for (name, provider) in manager.iter() {
        let provider_router = Router::new()
            .fallback(handlers::proxy::provider_proxy_handler)
            .with_state(Arc::clone(provider));
        router = router.nest(&format!("/{name}"), provider_router);
    }

    let manager = Arc::new(manager);
    router
        .fallback(handlers::proxy::proxy_handler)
        .with_state(manager)
        .layer(TraceLayer::new_for_http())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = config::Config::load(&args.config).unwrap_or_else(|e| {
        eprintln!("failed to load config: {e}");
        std::process::exit(1);
    });

    logging::init(&config.lacuna.logging).unwrap_or_else(|e| {
        eprintln!("failed to initialize logging: {e}");
        std::process::exit(1);
    });

    info!(count = config.providers.len(), "loaded providers");

    let mut manager = ProviderManager::new();
    for (key, provider_config) in &config.providers {
        let provider = provider::Provider::from_config(provider_config).unwrap_or_else(|e| {
            error!(provider = %key, %e, "failed to configure provider");
            std::process::exit(1);
        });
        manager.add(key.clone(), provider);
    }
    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port))
        .await
        .unwrap();
    info!(addr = %listener.local_addr().unwrap(), "listening");
    axum::serve(listener, app(manager)).await.unwrap();
}
