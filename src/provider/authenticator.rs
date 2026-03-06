use crate::config;

pub(super) trait ProviderAuthenticator {
    fn authenticate(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder;
}

struct NoAuth;

impl ProviderAuthenticator for NoAuth {
    fn authenticate(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }
}

struct BearerAuth {
    key: String,
}

impl ProviderAuthenticator for BearerAuth {
    fn authenticate(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.bearer_auth(&self.key)
    }
}

struct ApiKeyAuth {
    header: String,
    key: String,
}

impl ProviderAuthenticator for ApiKeyAuth {
    fn authenticate(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header(&self.header, &self.key)
    }
}

pub(super) fn build_authenticator(
    authorization: &config::Authorization,
    apikey: &str,
) -> Box<dyn ProviderAuthenticator + Send + Sync> {
    match authorization {
        config::Authorization::None => Box::new(NoAuth),
        config::Authorization::Bearer => Box::new(BearerAuth {
            key: apikey.to_owned(),
        }),
        config::Authorization::XApiKey => Box::new(ApiKeyAuth {
            header: "x-api-key".to_owned(),
            key: apikey.to_owned(),
        }),
        config::Authorization::XGoogApiKey => Box::new(ApiKeyAuth {
            header: "x-goog-api-key".to_owned(),
            key: apikey.to_owned(),
        }),
    }
}
