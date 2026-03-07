mod auth;
mod config;
mod handlers;
mod logging;
mod provider;
mod trace;

use axum::{Router, response::Redirect, routing::get};
use clap::Parser;
use provider::ProviderManager;
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

#[cfg(test)]
pub(crate) fn app(manager: ProviderManager, assets_path: &Path) -> Router {
    app_with_identity_header(manager, assets_path, None)
}

pub(crate) fn app_with_identity_header(
    manager: ProviderManager,
    assets_path: &Path,
    identity_header: Option<String>,
) -> Router {
    let mut router = Router::new()
        .route("/health", get(handlers::health::health))
        .nest("/ui", handlers::ui::router(assets_path))
        .route("/", get(|| async { Redirect::permanent("/ui") }));

    for (name, provider) in manager.iter() {
        let provider_router = Router::new()
            .fallback(handlers::proxy::provider_proxy_handler)
            .with_state(Arc::clone(provider));
        router = router.nest(&format!("/{name}"), provider_router);
    }

    let manager = Arc::new(manager);
    let mut router = router
        .fallback(handlers::proxy::proxy_handler)
        .with_state(manager)
        .layer(trace::layer());

    if let Some(header_name) = identity_header {
        router = router.layer(axum::middleware::from_fn(move |request, next| {
            auth::identity_middleware(header_name.clone(), request, next)
        }));
    }

    router
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
    axum::serve(
        listener,
        app_with_identity_header(manager, &args.assets, config.lacuna.identity_header),
    )
    .await
    .unwrap();
}
