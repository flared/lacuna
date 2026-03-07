use axum::{Router, response::Redirect, routing::get};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::auth;
use crate::handlers;
use crate::provider::ProviderManager;
use crate::trace;

pub struct AppBuilder {
    manager: Option<ProviderManager>,
    assets_path: Option<PathBuf>,
    identity_header: Option<String>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            manager: None,
            assets_path: None,
            identity_header: None,
        }
    }

    pub fn manager(mut self, manager: ProviderManager) -> Self {
        self.manager = Some(manager);
        self
    }

    pub fn assets_path(mut self, path: &Path) -> Self {
        self.assets_path = Some(path.to_owned());
        self
    }

    pub fn identity_header(mut self, header: Option<String>) -> Self {
        self.identity_header = header;
        self
    }

    pub fn build(self) -> Router {
        let manager = self.manager.unwrap_or_default();
        let assets_path = self.assets_path.unwrap_or_else(|| PathBuf::from("assets"));

        let mut router = Router::new()
            .route("/health", get(handlers::health::health))
            .route("/metrics", get(handlers::metrics::handler))
            .nest("/ui", handlers::ui::router(&assets_path))
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

        if let Some(header_name) = self.identity_header {
            router = router.layer(axum::middleware::from_fn(move |request, next| {
                auth::identity_middleware(header_name.clone(), request, next)
            }));
        }

        router
    }
}
