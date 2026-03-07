use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use serde::Serialize;
use tower_http::services::{ServeDir, ServeFile};

use crate::provider::ProviderManager;

#[derive(Serialize)]
struct UiProvider {
    name: String,
    baseurl: String,
}

async fn config(State(manager): State<Arc<ProviderManager>>) -> Json<HashMap<String, UiProvider>> {
    let providers = manager
        .iter()
        .map(|(key, provider)| {
            (
                key.clone(),
                UiProvider {
                    name: provider.name.clone(),
                    baseurl: provider.baseurl.as_str().to_owned(),
                },
            )
        })
        .collect();
    Json(providers)
}

pub fn router(assets_path: &Path) -> Router<Arc<ProviderManager>> {
    Router::new()
        .route("/config", get(config))
        .route_service("/", ServeFile::new(assets_path.join("index.html")))
        .fallback_service(ServeDir::new(assets_path))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config;
    use crate::provider::{self, ProviderManager};

    fn make_provider(key: &str, baseurl: &str) -> provider::Provider {
        provider::Provider::from_config(
            key,
            &config::Provider {
                name: key.to_owned(),
                description: String::new(),
                baseurl: baseurl.to_owned(),
                models: vec![],
                apikey: String::new(),
                authorization: config::Authorization::None,
                tailnet: false,
                compatibility: config::Compatibility::default(),
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_config() {
        let mut manager = ProviderManager::new();
        manager.add(make_provider("test-provider", "https://api.example.com"));

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
            .oneshot(
                Request::builder()
                    .uri("/ui/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let providers: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(providers.len(), 1);
        let provider = &providers["test-provider"];
        assert_eq!(provider["name"], "test-provider");
        assert_eq!(provider["baseurl"], "https://api.example.com/");
        assert!(provider.get("apikey").is_none());
    }

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
        assert!(html.contains("<html>"));
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
