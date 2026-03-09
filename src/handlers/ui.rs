use std::path::Path;
use std::sync::Arc;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

use crate::provider::ProviderManager;

pub fn router(assets_path: &Path) -> Router<Arc<ProviderManager>> {
    Router::new()
        .route_service("/", ServeFile::new(assets_path.join("index.html")))
        .fallback_service(ServeDir::new(assets_path))
}
