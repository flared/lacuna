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

async fn provider_proxy_handler(
    State(provider): State<Arc<provider::Provider>>,
    request: axum::extract::Request,
) -> Response {
    forward_to_provider(&provider, request).await
}

async fn proxy_handler(
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

fn app(manager: ProviderManager) -> Router {
    let mut router = Router::new().route("/health", get(health));

    for (name, provider) in manager.iter() {
        let provider_router = Router::new()
            .fallback(provider_proxy_handler)
            .with_state(Arc::clone(provider));
        router = router.nest(&format!("/{name}"), provider_router);
    }

    let manager = Arc::new(manager);
    router.fallback(proxy_handler).with_state(manager)
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
    async fn health_returns_ok() {
        let response = app(ProviderManager::new())
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
        let response = app(ProviderManager::new())
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
        manager.add(make_provider("test", &format!("http://{addr}"), compat));

        let response = app(manager)
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

        let response = app(manager)
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
