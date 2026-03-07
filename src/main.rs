mod config;
mod logging;
mod provider;

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Router, routing::get};
use clap::Parser;
use provider::ProviderManager;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info};

#[derive(Parser)]
struct Args {
    /// Path to the providers config file (YAML or JSON)
    #[arg(long)]
    config: PathBuf,

    /// Host to listen on
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

async fn health() -> &'static str {
    "ok"
}

async fn proxy_handler(
    State(manager): State<Arc<ProviderManager>>,
    request: axum::extract::Request,
) -> Response {
    let method = request.method().to_owned();
    let path = request.uri().path().to_owned();
    debug!(%method, %path, "downstream_req");
    let provider = match manager.get_for_path(&path) {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "no provider found for path").into_response(),
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

    let upstream_res = match upstream_req.send().await {
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

fn app(manager: Arc<ProviderManager>) -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(proxy_handler)
        .with_state(manager)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = config::Config::load(&args.config).unwrap_or_else(|e| {
        eprintln!("failed to load config: {e}");
        std::process::exit(1);
    });

    logging::init(&config.lacuna.logging).unwrap_or_else(|e| {
        eprintln!("failed to initialize logging: {e}");
        std::process::exit(1);
    });

    info!(count = config.providers.len(), "loaded providers");

    let mut manager = ProviderManager::new();
    for (key, provider_config) in &config.providers {
        let provider = provider::Provider::from_config(provider_config).unwrap_or_else(|e| {
            error!(provider = %key, %e, "failed to configure provider");
            std::process::exit(1);
        });
        manager.add(provider);
    }
    let manager = Arc::new(manager);

    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port))
        .await
        .unwrap();
    info!(addr = %listener.local_addr().unwrap(), "listening");
    axum::serve(listener, app(manager)).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn empty_manager() -> Arc<ProviderManager> {
        Arc::new(ProviderManager::new())
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let response = app(empty_manager())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"ok");
    }

    #[tokio::test]
    async fn unmatched_path_returns_404() {
        let response = app(empty_manager())
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
        // Spin up a mock upstream server.
        let upstream = Router::new().fallback(|request: axum::extract::Request| async move {
            let auth = request
                .headers()
                .get("authorization")
                .map(|v| v.to_str().unwrap().to_owned())
                .unwrap_or_default();
            let custom = request
                .headers()
                .get("x-custom-header")
                .map(|v| v.to_str().unwrap().to_owned())
                .unwrap_or_default();
            let path = request.uri().path().to_owned();
            let body = axum::body::to_bytes(request.into_body(), usize::MAX)
                .await
                .unwrap();
            let mut headers = axum::http::HeaderMap::new();
            headers.insert("x-test-header", "hello".parse().unwrap());
            headers.insert("x-received-auth", auth.parse().unwrap());
            headers.insert("x-received-custom", custom.parse().unwrap());
            (
                StatusCode::OK,
                headers,
                format!("echoed {} {}", path, String::from_utf8_lossy(&body)),
            )
        });
        let upstream_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = upstream_listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(upstream_listener, upstream).await.unwrap();
        });

        // Configure a provider pointing at the mock upstream.
        let mut manager = ProviderManager::new();
        let mut compat = provider::compatibility::Compatibility::default();
        compat.openai_chat = true;
        let provider = provider::Provider::from_config(&config::Provider {
            name: "test".to_owned(),
            description: String::new(),
            baseurl: format!("http://{upstream_addr}"),
            models: vec![],
            apikey: "sk-test-key".to_owned(),
            authorization: config::Authorization::Bearer,
            tailnet: false,
            compatibility: compat,
        })
        .unwrap();
        manager.add(provider);

        let response = app(Arc::new(manager))
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/chat/completions")
                    .header("x-custom-header", "custom-value")
                    .body(Body::from("test-body"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-test-header").unwrap(), "hello");
        assert_eq!(
            response.headers().get("x-received-auth").unwrap(),
            "Bearer sk-test-key"
        );
        assert_eq!(
            response.headers().get("x-received-custom").unwrap(),
            "custom-value"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "echoed /v1/chat/completions test-body"
        );
    }
}
