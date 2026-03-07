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

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_ui_index() {
        let response = crate::app::AppBuilder::new()
            .build()
            .oneshot(Request::builder().uri("/ui").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("<html"));
    }

    #[tokio::test]
    async fn test_root_redirects_to_ui() {
        let response = crate::app::AppBuilder::new()
            .build()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(response.headers().get("location").unwrap(), "/ui");
    }
}
