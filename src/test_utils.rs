use axum::Router;
use tokio::net::TcpListener;

use crate::config;
use crate::provider;

pub async fn spawn_echo_server() -> std::net::SocketAddr {
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

pub fn make_provider(
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
