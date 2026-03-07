use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn handler() -> impl IntoResponse {
    let body = crate::metrics::render();
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use std::path::Path;

    use crate::provider::{self, ProviderManager};
    use crate::test_utils::{make_provider, spawn_echo_server};

    #[tokio::test]
    async fn metrics_counts_requests_per_provider() {
        crate::metrics::init();
        let addr = spawn_echo_server().await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "test-metrics",
            &format!("http://{addr}"),
            compat,
        ));

        let app = crate::app(manager, Path::new("assets"));

        // Make a proxy request to the provider.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Hit the metrics endpoint.
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();

        assert!(
            body.contains("lacuna_provider_requests_total"),
            "metrics should contain the request counter"
        );
        assert!(
            body.contains("provider=\"test-metrics\""),
            "metrics should contain the provider label"
        );
        assert!(
            body.contains("user=\"\""),
            "metrics should contain an empty user label when no identity is set"
        );
    }
}
