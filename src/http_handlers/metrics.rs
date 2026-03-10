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
    use crate::test_utils::{make_provider, spawn_echo_server, spawn_fixed_response_server};

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
    async fn metrics_records_token_usage() {
        crate::metrics::init();
        let addr = spawn_fixed_response_server(
            r#"{"usage": {"prompt_tokens": 10, "completion_tokens": 20}}"#,
        )
        .await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "test-tokens",
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

        // Consume the response body so InspectingStream fires the on_complete callback.
        let _ = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();

        let body = get_metrics_body(app).await;
        assert!(
            body.contains(
                r#"lacuna_provider_input_tokens_total{provider="test-tokens",handler="openai_chat_completion",user=""} 10"#
            ),
            "expected input tokens metric line, got:\n{body}"
        );
        assert!(
            body.contains(
                r#"lacuna_provider_output_tokens_total{provider="test-tokens",handler="openai_chat_completion",user=""} 20"#
            ),
            "expected output tokens metric line, got:\n{body}"
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
