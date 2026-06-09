use crate::config;

#[derive(Debug, thiserror::Error)]
pub enum AuthenticatorError {
    #[error("invalid bearer token: {0}")]
    InvalidBearerToken(reqwest::header::InvalidHeaderValue),
    #[error("invalid api key for header {header}: {source}")]
    InvalidApiKey {
        header: reqwest::header::HeaderName,
        source: reqwest::header::InvalidHeaderValue,
    },
}

pub(super) trait ProviderAuthenticator: std::fmt::Debug {
    fn authenticate(&self, request: &mut reqwest::Request) -> Result<(), AuthenticatorError>;
}

#[derive(Debug)]
struct NoAuth;

impl ProviderAuthenticator for NoAuth {
    fn authenticate(&self, _request: &mut reqwest::Request) -> Result<(), AuthenticatorError> {
        Ok(())
    }
}

#[derive(Debug)]
struct BearerAuth {
    key: String,
}

impl ProviderAuthenticator for BearerAuth {
    fn authenticate(&self, request: &mut reqwest::Request) -> Result<(), AuthenticatorError> {
        let value = format!("Bearer {}", self.key)
            .parse()
            .map_err(AuthenticatorError::InvalidBearerToken)?;
        request
            .headers_mut()
            .insert(reqwest::header::AUTHORIZATION, value);
        Ok(())
    }
}

#[derive(Debug)]
struct ApiKeyAuth {
    header: reqwest::header::HeaderName,
    key: String,
}

impl ProviderAuthenticator for ApiKeyAuth {
    fn authenticate(&self, request: &mut reqwest::Request) -> Result<(), AuthenticatorError> {
        let value = self
            .key
            .parse()
            .map_err(|source| AuthenticatorError::InvalidApiKey {
                header: self.header.clone(),
                source,
            })?;
        request.headers_mut().insert(self.header.clone(), value);
        Ok(())
    }
}

pub(super) fn build_authenticator(
    authorization: Option<&config::Authorization>,
) -> Box<dyn ProviderAuthenticator + Send + Sync> {
    match authorization {
        None => Box::new(NoAuth),
        Some(config::Authorization::Bearer { apikey }) => Box::new(BearerAuth {
            key: apikey.clone(),
        }),
        Some(config::Authorization::XApiKey { apikey }) => Box::new(ApiKeyAuth {
            header: reqwest::header::HeaderName::from_static("x-api-key"),
            key: apikey.clone(),
        }),
        Some(config::Authorization::XGoogApiKey { apikey }) => Box::new(ApiKeyAuth {
            header: reqwest::header::HeaderName::from_static("x-goog-api-key"),
            key: apikey.clone(),
        }),
    }
}
