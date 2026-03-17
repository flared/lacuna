use axum::{Router, response::Redirect, routing::get};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::http_handlers;
use crate::http_middleware::auth;
use crate::http_middleware::capabilities;
use crate::http_middleware::user_agent;
use crate::provider::ProviderManager;
use crate::trace;
use crate::user_agent::{UserAgentExtractor, UserAgentPatternConfig};

#[derive(Debug, Default)]
pub struct AppBuilder {
    manager: Option<ProviderManager>,
    assets_path: Option<PathBuf>,
    identity_header: Option<String>,
    capabilities_header: Option<String>,
    user_agents: Vec<UserAgentPatternConfig>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self {
            manager: None,
            assets_path: None,
            identity_header: None,
            capabilities_header: None,
            user_agents: Vec::new(),
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

    pub fn capabilities_header(mut self, header: Option<String>) -> Self {
        self.capabilities_header = header;
        self
    }

    pub fn user_agents(mut self, configs: Vec<UserAgentPatternConfig>) -> Self {
        self.user_agents = configs;
        self
    }

    pub fn build(self) -> Router {
        let manager = self.manager.unwrap_or_default();
        let assets_path = self
            .assets_path
            .unwrap_or_else(|| PathBuf::from("frontend/dist"));

        let mut router = Router::new()
            .route("/health", get(http_handlers::health::health))
            .route("/metrics", get(http_handlers::metrics::handler))
            .nest("/api", http_handlers::api::router())
            .nest("/ui/", http_handlers::ui::router(&assets_path))
            .route("/", get(|| async { Redirect::permanent("/ui/") }));

        for (name, provider) in manager.iter() {
            let provider_router = Router::new()
                .fallback(http_handlers::proxy::provider_proxy_handler)
                .with_state(Arc::clone(provider));
            router = router.nest(&format!("/{name}"), provider_router);
        }

        let manager = Arc::new(manager);
        let mut router = router
            .fallback(http_handlers::proxy::proxy_handler)
            .with_state(manager)
            .layer(trace::layer());

        let extractor = Arc::new(UserAgentExtractor::new(self.user_agents));
        router = router.layer(axum::middleware::from_fn_with_state(
            extractor,
            user_agent::user_agent_middleware,
        ));

        if let Some(header_name) = self.identity_header {
            router = router.layer(axum::middleware::from_fn_with_state(
                header_name,
                auth::identity_middleware,
            ));
        }

        if let Some(header_name) = self.capabilities_header {
            router = router.layer(axum::middleware::from_fn_with_state(
                header_name,
                capabilities::capabilities_middleware,
            ));
        }

        router
    }
}
