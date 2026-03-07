mod authenticator;
pub mod compatibility;
mod manager;

use crate::config;
use authenticator::{ProviderAuthenticator, build_authenticator};
use compatibility::Compatibility;

pub use manager::ProviderManager;

pub struct Provider {
    #[allow(dead_code)]
    pub name: String,
    baseurl: reqwest::Url,
    client: reqwest::Client,
    authenticator: Box<dyn ProviderAuthenticator + Send + Sync>,
    pub compatibility: Compatibility,
}

impl Provider {
    pub fn from_config(config: &config::Provider) -> Result<Self, anyhow::Error> {
        let baseurl = reqwest::Url::parse(&config.baseurl)?;
        let authenticator = build_authenticator(&config.authorization, &config.apikey);
        Ok(Self {
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
    ) -> Result<reqwest::RequestBuilder, anyhow::Error> {
        let method = incoming.method().clone();
        let uri = incoming.uri().clone();

        let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");

        let url = self.baseurl.join(path_and_query)?;

        let body = reqwest::Body::wrap_stream(incoming.into_body().into_data_stream());

        let request = self.client.request(method, url).body(body);
        Ok(self.authenticator.authenticate(request))
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
        Provider::from_config(&config::Provider {
            name: String::new(),
            description: String::new(),
            baseurl: baseurl.to_owned(),
            models: vec!["model-1".to_owned()],
            apikey: apikey.to_owned(),
            authorization,
            tailnet: false,
            compatibility: config::Compatibility::default(),
        })
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
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(req.url().as_str(), "https://api.anthropic.com/v1/messages");
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
            .unwrap()
            .build()
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
            .unwrap()
            .build()
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
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(req.headers().get("x-goog-api-key").unwrap(), "goog-key");
    }

    #[tokio::test]
    async fn no_auth() {
        let provider = test_provider("https://example.com", config::Authorization::None, "");
        let req = provider
            .build_request(incoming_request("GET", "/health", Body::empty()))
            .unwrap()
            .build()
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
            .unwrap()
            .build()
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
            .unwrap()
            .build()
            .unwrap();
        assert_eq!(req.method(), "POST");
        assert_eq!(req.url().as_str(), "https://api.example.com/v1/chat");
        // The body is a stream so as_bytes() is None, but we can verify it
        // is present (not None).
        assert!(req.body().is_some());
    }
}
