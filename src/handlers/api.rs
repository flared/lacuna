use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::response::Json;
use axum::routing::get;
use serde::Serialize;

use crate::provider::ProviderManager;

#[derive(Serialize)]
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
    Router::new().route("/config", get(api_config))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::provider::ProviderManager;
    use crate::test_utils::make_provider;

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
        let providers: std::collections::HashMap<String, serde_json::Value> =
            serde_json::from_slice(&body).unwrap();

        assert_eq!(providers.len(), 1);
        let provider = &providers["test-provider"];
        assert_eq!(provider["name"], "test-provider");
        assert_eq!(provider["baseurl"], "https://api.example.com/");
        assert!(provider.get("apikey").is_none());
    }
}
