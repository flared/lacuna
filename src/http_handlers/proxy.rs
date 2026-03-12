use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use std::sync::Arc;
use tracing::{debug, info, warn};

use crate::api_type::{ApiTypeHandler, api_type_for_path};

use crate::auth;
use crate::inspecting_stream::InspectingStream;
use crate::metrics;
use crate::provider::{self, ProviderManager};
use crate::request_metadata::RequestMetadata;

async fn forward_to_provider(
    provider: &provider::Provider,
    request: axum::extract::Request,
) -> Response {
    let method = request.method().to_owned();
    let path = request.uri().path().to_owned();
    let user = auth::get_caller_identity(&request);
    let api_type_handler = api_type_for_path(&path).map(|t| t.handler());
    let api_type_handler_id = api_type_handler
        .as_ref()
        .map(|h| h.id().to_owned())
        .unwrap_or_default();

    let request_metadata = RequestMetadata {
        provider_key: provider.key.clone(),
        api_handler_id: api_type_handler_id,
        user_identity: user,
    };

    metrics::record_request(&request_metadata);
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
    let status_code = status.as_u16();

    info!(%method, %path, %status, "upstream_resp");

    let inspect_headers = headers.clone();
    let stream = InspectingStream::new(upstream_res.bytes_stream(), move |body: Bytes| {
        on_response_stream_complete(
            body,
            status_code,
            inspect_headers,
            api_type_handler,
            &request_metadata,
        );
    });

    let body = Body::from_stream(stream);

    let mut builder = Response::builder().status(status_code);
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

fn on_response_stream_complete(
    body: Bytes,
    status_code: u16,
    headers: http::HeaderMap,
    api_type_handler: Option<Box<dyn ApiTypeHandler + Send>>,
    request_metadata: &RequestMetadata,
) {
    let api_type_handler = match api_type_handler {
        Some(handler) => handler,
        None => return,
    };

    let mut response_builder = http::Response::builder().status(status_code);
    for (name, value) in headers.iter() {
        response_builder = response_builder.header(name, value);
    }
    let response = match response_builder.body(body) {
        Ok(r) => r,
        Err(e) => {
            warn!("Failed to build response: {e}");
            return;
        }
    };

    let response_metadata = match api_type_handler.inspect_response(&response) {
        Ok(metadata) => metadata,
        Err(e) => {
            warn!("Failed to inspect response: {e}");
            return;
        }
    };
    metrics::record_response(request_metadata, &response_metadata);
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::provider::{self, ProviderManager};
    use crate::test_utils::{make_provider, spawn_echo_server};

    #[tokio::test]
    async fn unmatched_path_returns_404() {
        let response = crate::app::AppBuilder::new()
            .build()
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
        manager.add(make_provider(
            "provider-key",
            &format!("http://{addr}"),
            compat,
        ));

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
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
        manager.add(make_provider("myopenai", &format!("http://{addr}"), compat));

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
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
