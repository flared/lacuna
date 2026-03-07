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
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tokio::net::TcpListener;
    use tower::ServiceExt;

    use std::path::Path;

    use crate::config;
    use crate::provider::{self, ProviderManager};

    async fn spawn_echo_server() -> std::net::SocketAddr {
        let upstream = Router::new().fallback(|request: axum::extract::Request| async move {
            let path = request.uri().path().to_owned();
            let body = axum::body::to_bytes(request.into_body(), usize::MAX)
                .await
                .unwrap();
            format!("echoed {} {}", path, String::from_utf8_lossy(&body))
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        addr
    }

    fn make_provider(
        key: &str,
        baseurl: &str,
        compat: provider::compatibility::Compatibility,
    ) -> provider::Provider {
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
                compatibility: compat,
            },
        )
        .unwrap()
    }

    #[tokio::test]
    async fn metrics_counts_requests_per_provider() {
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
