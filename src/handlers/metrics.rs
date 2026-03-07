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

    use crate::app::AppBuilder;
    use crate::provider::{self, ProviderManager};
    use crate::test_utils::{make_provider, spawn_echo_server};

    async fn get_metrics_body(app: axum::Router) -> String {
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
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn metrics_counts_anonymous_request() {
        crate::metrics::init();
        let addr = spawn_echo_server().await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "test-anon",
            &format!("http://{addr}"),
            compat,
        ));

        let app = AppBuilder::new().manager(manager).build();

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

        let body = get_metrics_body(app).await;
        assert!(
            body.contains(r#"lacuna_provider_requests_total{provider="test-anon",user=""} 1"#),
            "expected anonymous request metric line, got:\n{body}"
        );
    }

    #[tokio::test]
    async fn metrics_counts_identified_request() {
        crate::metrics::init();
        let addr = spawn_echo_server().await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "test-identified",
            &format!("http://{addr}"),
            compat,
        ));

        let app = AppBuilder::new()
            .manager(manager)
            .identity_header(Some("X-User-Email".to_owned()))
            .build();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("X-User-Email", "alice@example.com")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = get_metrics_body(app).await;
        assert!(
            body.contains(
                r#"lacuna_provider_requests_total{provider="test-identified",user="alice@example.com"} 1"#
            ),
            "expected identified request metric line, got:\n{body}"
        );
    }
}
