use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;
use tracing::{debug, info};

use crate::provider::{self, ProviderManager};

async fn forward_to_provider(
    provider: &provider::Provider,
    request: axum::extract::Request,
) -> Response {
    let method = request.method().to_owned();
    let path = request.uri().path().to_owned();
    debug!(%method, %path, "downstream_req");

    let upstream_req = match provider.build_request(request) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("failed to build request: {e}"),
            )
                .into_response();
        }
    };

    let upstream_res = match provider.send(upstream_req).await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("upstream request failed: {e}"),
            )
                .into_response();
        }
    };

    let status = upstream_res.status();
    let headers = upstream_res.headers().clone();
    let body = Body::from_stream(upstream_res.bytes_stream());

    info!(%method, %path, %status, "upstream_resp");

    let mut builder = Response::builder().status(status.as_u16());
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    builder.body(body).unwrap().into_response()
}

pub async fn provider_proxy_handler(
    State(provider): State<Arc<provider::Provider>>,
    request: axum::extract::Request,
) -> Response {
    forward_to_provider(&provider, request).await
}

pub async fn proxy_handler(
    State(manager): State<Arc<ProviderManager>>,
    request: axum::extract::Request,
) -> Response {
    let path = request.uri().path().to_owned();
    let provider = match manager.get_for_path(&path) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "no provider found for path").into_response(),
    };
    forward_to_provider(provider, request).await
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

    /// Spawn an echo server that returns the path and body it received.
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
        name: &str,
        baseurl: &str,
        compat: provider::compatibility::Compatibility,
    ) -> provider::Provider {
        provider::Provider::from_config(&config::Provider {
            name: name.to_owned(),
            description: String::new(),
            baseurl: baseurl.to_owned(),
            models: vec![],
            apikey: String::new(),
            authorization: config::Authorization::None,
            tailnet: false,
            compatibility: compat,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn unmatched_path_returns_404() {
        let response = crate::app(ProviderManager::new(), Path::new("assets"))
            .oneshot(
                Request::builder()
                    .uri("/v1/chat/completions")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn proxy_forwards_to_upstream() {
        let addr = spawn_echo_server().await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(
            "provider-key".to_owned(),
            make_provider("provider-name", &format!("http://{addr}"), compat),
        );

        let response = crate::app(manager, Path::new("assets"))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .body(Body::from("test-body"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "echoed /v1/chat/completions test-body"
        );
    }

    #[tokio::test]
    async fn proxy_routes_by_provider_name_prefix() {
        let addr = spawn_echo_server().await;

        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;

        let mut manager = ProviderManager::new();
        manager.add(
            "myopenai".to_owned(),
            make_provider("My OpenAI Provider", &format!("http://{addr}"), compat),
        );

        let response = crate::app(manager, Path::new("assets"))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/myopenai/v1/chat/completions")
                    .body(Body::from("hello"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "echoed /v1/chat/completions hello"
        );
    }
}
