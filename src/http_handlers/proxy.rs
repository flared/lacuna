use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

use crate::api_type::{ApiType, api_type_for_path};

use crate::capabilities::Capabilities;
use crate::http_middleware::{auth, capabilities};
use crate::inspector::CallbackInspector;
use crate::inspector::stream::InspectorStream;
use crate::metrics;
use crate::provider::{self, ProviderManager};
use crate::request_metadata::{RequestInspectionMetadata, RequestMetadata};

async fn forward_to_provider(
    provider: &provider::Provider,
    api_type: Option<&ApiType>,
    request: axum::extract::Request,
) -> Response {
    if let Some(caps) = capabilities::get_capabilities(&request)
        && !caps.is_provider_allowed(&provider.key)
    {
        return capabilities_forbidden_response(
            &format!("provider '{}' is not allowed", &provider.key),
            &caps,
        );
    }

    let method = request.method().to_owned();
    let path = request.uri().path().to_owned();
    let user = auth::get_caller_identity(&request);
    let api_type_handler = api_type.map(|t| t.handler());
    let api_type_handler_id = api_type_handler
        .as_ref()
        .map(|h| h.id())
        .unwrap_or_default() // empty string: ""
        .to_owned();

    let mut request_metadata = RequestMetadata {
        provider_key: provider.key.clone(),
        api_handler_id: api_type_handler_id,
        user_identity: user,
        inspected: None,
    };

    debug!(%method, %path, "downstream_req");

    // Create a request inspector to extract request metadata.
    // Result will be stored in `request_inspection_metadata` after the request
    // has been sent.
    let request_inspection_metadata: Arc<Mutex<Option<RequestInspectionMetadata>>> =
        Arc::new(Mutex::new(None));

    let request = if let Some(api_type_handler) = &api_type_handler {
        let inspector = api_type_handler.request_inspector();
        let slot = Arc::clone(&request_inspection_metadata);
        let inspector = CallbackInspector::new(inspector, move |result| match result {
            Ok(metadata) => {
                debug!(?metadata, "request_inspection");
                match slot.lock() {
                    Ok(mut guard) => {
                        *guard = Some(metadata.clone());
                    }
                    Err(e) => error!("Failed to lock request inspection metadata: {e}"),
                }
            }
            Err(e) => warn!("Failed to inspect request: {e}"),
        });
        let (parts, body) = request.into_parts();
        let stream = InspectorStream::new(body.into_data_stream(), Box::new(inspector));
        axum::http::Request::from_parts(parts, Body::from_stream(stream))
    } else {
        request
    };

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

    request_metadata.inspected = match request_inspection_metadata.lock() {
        Ok(mut guard) => guard.take(),
        Err(e) => {
            error!("Failed to lock request inspection metadata: {e}");
            None
        }
    };

    let model = request_metadata
        .inspected
        .as_ref()
        .and_then(|m| m.model.as_deref());
    info!(%method, %path, %status, ?model, "upstream_resp");
    metrics::record_request(&request_metadata);

    let body = if let Some(api_type_handler) = api_type_handler {
        let inspector = api_type_handler.response_inspector(status_code, &headers);
        let inspector = CallbackInspector::new(inspector, move |result| match result {
            Ok(metadata) => metrics::record_response(&request_metadata, metadata),
            Err(e) => warn!("Failed to inspect response: {e}"),
        });
        let stream = InspectorStream::new(upstream_res.bytes_stream(), Box::new(inspector));
        Body::from_stream(stream)
    } else {
        Body::from_stream(upstream_res.bytes_stream())
    };

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
    let path = request.uri().path().to_owned();
    let api_type = api_type_for_path(&path);
    forward_to_provider(&provider, api_type.as_ref(), request).await
}

pub async fn proxy_handler(
    State(manager): State<Arc<ProviderManager>>,
    request: axum::extract::Request,
) -> Response {
    let path = request.uri().path().to_owned();
    let api_type = match api_type_for_path(&path) {
        Some(api_type) => api_type,
        None => return (StatusCode::NOT_FOUND, "unknown api type for path").into_response(),
    };
    let provider = match manager.get_for_api_type(&api_type) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "no provider found for api type").into_response(),
    };
    forward_to_provider(provider, Some(&api_type), request).await
}

fn capabilities_forbidden_response(error: &str, capabilities: &Capabilities) -> Response {
    let body = serde_json::json!({
        "error": error,
        "capabilities": capabilities,
    });
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::provider::ProviderManager;
    use crate::provider::compatibility::Compatibility;
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

        let mut compat = Compatibility::default();
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

        let mut compat = Compatibility::default();
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

    #[tokio::test]
    async fn proxy_forwards_to_provider_without_api_type() {
        // We should always be able to call /myprovider/<path> without specifying an API type.
        // This is useful for very simple lacuna use cases where we just want to proxy a generic
        // HTTP API that may not even be an AI provider.
        let addr = spawn_echo_server().await;

        // This is a default compatibility where everything is disabled.
        let compat = Compatibility::default();

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "myprovider",
            &format!("http://{addr}"),
            compat,
        ));

        let response = crate::app::AppBuilder::new()
            .manager(manager)
            .build()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/myprovider/some/unknown/path")
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
            "echoed /some/unknown/path hello"
        );
    }

    #[tokio::test]
    async fn proxy_validates_capabilities() {
        let addr = spawn_echo_server().await;

        let mut manager = ProviderManager::new();
        manager.add(make_provider(
            "myprovider",
            &format!("http://{addr}"),
            Compatibility::default(),
        ));

        let app = crate::app::AppBuilder::new()
            .manager(manager)
            .capabilities_header(Some("Tailscale-App-Capabilities".to_owned()))
            .build();

        // Request without the capabilities header — should be forbidden.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/myprovider/endpoint")
                    .body(Body::from("test"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "error": "provider 'myprovider' is not allowed",
                "capabilities":  {"providers": [] }
            })
        );

        // Request with the capabilities header granting access — should succeed.
        let caps_header = serde_json::json!({
            "flare.io/cap/lacuna": [
                {"providers": ["myprovider"] }
            ]
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/myprovider/endpoint")
                    .header("Tailscale-App-Capabilities", caps_header.to_string())
                    .body(Body::from("test"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
