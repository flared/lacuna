mod authenticator;
pub mod compatibility;
mod manager;

use crate::config;
use authenticator::{ProviderAuthenticator, build_authenticator};
use compatibility::Compatibility;

pub use manager::ProviderManager;

const HEADERS_TO_STRIP: &[&str] = &[
    // Common hop-by-hop headers that should be stripped.
    "host",
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailers",
    "transfer-encoding",
    "upgrade",
    // Tailscale specific headers.
    "tailscale-.*",
];

static HEADERS_TO_STRIP_SET: std::sync::LazyLock<regex::RegexSet> =
    std::sync::LazyLock::new(|| {
        let anchored: Vec<String> = HEADERS_TO_STRIP.iter().map(|p| format!("^{p}$")).collect();
        regex::RegexSet::new(anchored).expect("header strip patterns should be valid")
    });

fn strip_hop_headers(mut headers: axum::http::HeaderMap) -> axum::http::HeaderMap {
    // Extract additional hop-by-hop headers declared in the Connection header.
    let connection_headers: Vec<axum::http::HeaderName> = headers
        .get_all("connection")
        .iter()
        .flat_map(|value| {
            value
                .to_str()
                .unwrap_or("")
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
        })
        .collect();

    for name in headers.keys().cloned().collect::<Vec<_>>() {
        if HEADERS_TO_STRIP_SET.is_match(name.as_str()) || connection_headers.contains(&name) {
            headers.remove(&name);
        }
    }
    headers
}

pub struct Provider {
    #[allow(dead_code)]
    pub key: String,
    #[allow(dead_code)]
    pub name: String,
    pub baseurl: reqwest::Url,
    client: reqwest::Client,
    authenticator: Box<dyn ProviderAuthenticator + Send + Sync>,
    pub compatibility: Compatibility,
}

impl Provider {
    pub fn from_config(key: &str, config: &config::Provider) -> Result<Self, anyhow::Error> {
        let baseurl = reqwest::Url::parse(&config.baseurl)?;
        let authenticator = build_authenticator(&config.authorization, &config.apikey);
        Ok(Self {
            key: key.to_owned(),
            name: config.name.clone(),
            baseurl,
            client: reqwest::Client::new(),
            authenticator,
            compatibility: config.compatibility.clone(),
        })
    }

    pub fn build_request(
        &self,
        incoming: axum::extract::Request,
    ) -> Result<reqwest::Request, anyhow::Error> {
        let method = incoming.method().clone();
        let uri = incoming.uri().clone();

        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

        // Strip the leading '/' so that Url::join resolves the path relative
        // to the base URL path instead of replacing it (RFC 3986 behaviour).
        let relative = path_and_query.strip_prefix('/').unwrap_or(path_and_query);
        let url = self.baseurl.join(relative)?;

        let (parts, body) = incoming.into_parts();
        let headers = strip_hop_headers(parts.headers);

        let body = reqwest::Body::wrap_stream(body.into_data_stream());

        let mut request = self
            .client
            .request(method, url)
            .headers(headers)
            .body(body)
            .build()?;
        self.authenticator.authenticate(&mut request)?;
        Ok(request)
    }

    pub async fn send(
        &self,
        request: reqwest::Request,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client.execute(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;

    fn test_provider(
        baseurl: &str,
        authorization: config::Authorization,
        apikey: &str,
    ) -> Provider {
        Provider::from_config(
            "test",
            &config::Provider {
                name: String::new(),
                description: String::new(),
                baseurl: baseurl.to_owned(),
                models: vec!["model-1".to_owned()],
                apikey: apikey.to_owned(),
                authorization,
                tailnet: false,
                compatibility: config::Compatibility::default(),
            },
        )
        .expect("test baseurl should be valid")
    }

    fn incoming_request(method: &str, uri: &str, body: Body) -> axum::extract::Request {
        Request::builder()
            .method(method)
            .uri(uri)
            .body(body)
            .unwrap()
    }

    #[tokio::test]
    async fn rewrites_base_url() {
        let provider = test_provider("https://api.anthropic.com", config::Authorization::None, "");
        let req = provider
            .build_request(incoming_request("GET", "/v1/messages", Body::empty()))
            .unwrap();
        assert_eq!(req.url().as_str(), "https://api.anthropic.com/v1/messages");
    }

    #[tokio::test]
    async fn preserves_base_url_path() {
        let provider = test_provider(
            "https://openrouter.ai/api/",
            config::Authorization::None,
            "",
        );
        let req = provider
            .build_request(incoming_request(
                "GET",
                "/v1/chat/completions",
                Body::empty(),
            ))
            .unwrap();
        assert_eq!(
            req.url().as_str(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
    }

    #[tokio::test]
    async fn bearer_auth() {
        let provider = test_provider(
            "https://api.example.com",
            config::Authorization::Bearer,
            "sk-test-key",
        );
        let req = provider
            .build_request(incoming_request("POST", "/v1/chat", Body::empty()))
            .unwrap();
        assert_eq!(
            req.headers().get("authorization").unwrap(),
            "Bearer sk-test-key"
        );
    }

    #[tokio::test]
    async fn x_api_key_auth() {
        let provider = test_provider(
            "https://api.anthropic.com",
            config::Authorization::XApiKey,
            "sk-ant-key",
        );
        let req = provider
            .build_request(incoming_request("POST", "/v1/messages", Body::empty()))
            .unwrap();
        assert_eq!(req.headers().get("x-api-key").unwrap(), "sk-ant-key");
        assert!(req.headers().get("authorization").is_none());
    }

    #[tokio::test]
    async fn x_goog_api_key_auth() {
        let provider = test_provider(
            "https://generativelanguage.googleapis.com",
            config::Authorization::XGoogApiKey,
            "goog-key",
        );
        let req = provider
            .build_request(incoming_request("POST", "/v1/models", Body::empty()))
            .unwrap();
        assert_eq!(req.headers().get("x-goog-api-key").unwrap(), "goog-key");
    }

    #[tokio::test]
    async fn no_auth() {
        let provider = test_provider("https://example.com", config::Authorization::None, "");
        let req = provider
            .build_request(incoming_request("GET", "/health", Body::empty()))
            .unwrap();
        assert!(req.headers().get("authorization").is_none());
        assert!(req.headers().get("x-api-key").is_none());
        assert!(req.headers().get("x-goog-api-key").is_none());
    }

    #[tokio::test]
    async fn preserves_path() {
        let provider = test_provider("https://api.example.com/", config::Authorization::None, "");
        let req = provider
            .build_request(incoming_request(
                "GET",
                "/v1/models?limit=10",
                Body::empty(),
            ))
            .unwrap();
        assert_eq!(
            req.url().as_str(),
            "https://api.example.com/v1/models?limit=10"
        );
    }

    #[tokio::test]
    async fn forwards_body() {
        let provider = test_provider("https://api.example.com", config::Authorization::None, "");
        let payload = b"{\"prompt\":\"hello\"}";
        let req = provider
            .build_request(incoming_request(
                "POST",
                "/v1/chat",
                Body::from(payload.to_vec()),
            ))
            .unwrap();
        assert_eq!(req.method(), "POST");
        assert_eq!(req.url().as_str(), "https://api.example.com/v1/chat");
        // The body is a stream so as_bytes() is None, but we can verify it
        // is present (not None).
        assert!(req.body().is_some());
    }

    #[test]
    fn strip_hop_headers_filters_correctly() {
        let mut headers = axum::http::HeaderMap::new();
        // Standard hop-by-hop headers.
        headers.insert("host", "example.com".parse().unwrap());
        headers.insert("connection", "x-custom-hop".parse().unwrap());
        headers.insert("transfer-encoding", "chunked".parse().unwrap());
        headers.insert("upgrade", "websocket".parse().unwrap());
        // Tailscale headers.
        headers.insert("tailscale-user-login", "user@example.com".parse().unwrap());
        headers.insert("tailscale-user-name", "User".parse().unwrap());
        // Connection-declared header.
        headers.insert("x-custom-hop", "value".parse().unwrap());
        // Headers that should be kept.
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("x-request-id", "abc123".parse().unwrap());

        let result = strip_hop_headers(headers);

        // Stripped.
        assert!(result.get("host").is_none());
        assert!(result.get("connection").is_none());
        assert!(result.get("transfer-encoding").is_none());
        assert!(result.get("upgrade").is_none());
        assert!(result.get("tailscale-user-login").is_none());
        assert!(result.get("tailscale-user-name").is_none());
        assert!(result.get("x-custom-hop").is_none());

        // Kept.
        assert_eq!(result.get("content-type").unwrap(), "application/json");
        assert_eq!(result.get("x-request-id").unwrap(), "abc123");
    }
}
