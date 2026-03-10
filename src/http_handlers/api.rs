use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use serde::{Deserialize, Serialize};

use crate::provider::ProviderManager;

#[derive(Serialize, Deserialize)]
struct ApiInfo {
    version: String,
    license: String,
}

async fn api_info() -> Json<ApiInfo> {
    Json(ApiInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        license: include_str!("../../LICENSE").to_owned(),
    })
}

#[derive(Serialize, Deserialize)]
struct ApiProvider {
    name: String,
    baseurl: String,
}

async fn api_config(
    State(manager): State<Arc<ProviderManager>>,
) -> Json<HashMap<String, ApiProvider>> {
    let providers = manager
        .iter()
        .map(|(key, provider)| {
            (
                key.clone(),
                ApiProvider {
                    name: provider.name.clone(),
                    baseurl: provider.baseurl.as_str().to_owned(),
                },
            )
        })
        .collect();
    Json(providers)
}

pub fn router() -> Router<Arc<ProviderManager>> {
    Router::new()
        .route("/info", get(api_info))
        .route("/config", get(api_config))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::provider::ProviderManager;
    use crate::test_utils::make_provider;

    use super::{ApiInfo, ApiProvider};

    #[tokio::test]
    async fn test_info() {
        let manager = ProviderManager::new();

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
            .oneshot(
                Request::builder()
                    .uri("/api/info")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let info: ApiInfo = serde_json::from_slice(&body).unwrap();

        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.license.contains("MIT License"));
    }

    #[tokio::test]
    async fn test_config() {
        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "test-provider",
            "https://api.example.com",
            Default::default(),
        ));

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let providers: std::collections::HashMap<String, ApiProvider> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(providers.len(), 1);
        let provider = &providers["test-provider"];
        assert_eq!(provider.name, "test-provider");
        assert_eq!(provider.baseurl, "https://api.example.com/");
    }
}
