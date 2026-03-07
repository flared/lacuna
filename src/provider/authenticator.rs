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

pub(super) trait ProviderAuthenticator {
    fn authenticate(&self, request: &mut reqwest::Request) -> Result<(), AuthenticatorError>;
}

struct NoAuth;

impl ProviderAuthenticator for NoAuth {
    fn authenticate(&self, _request: &mut reqwest::Request) -> Result<(), AuthenticatorError> {
        Ok(())
    }
}

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
    authorization: &config::Authorization,
    apikey: &str,
) -> Box<dyn ProviderAuthenticator + Send + Sync> {
    match authorization {
        config::Authorization::None => Box::new(NoAuth),
        config::Authorization::Bearer => Box::new(BearerAuth {
            key: apikey.to_owned(),
        }),
        config::Authorization::XApiKey => Box::new(ApiKeyAuth {
            header: reqwest::header::HeaderName::from_static("x-api-key"),
            key: apikey.to_owned(),
        }),
        config::Authorization::XGoogApiKey => Box::new(ApiKeyAuth {
            header: reqwest::header::HeaderName::from_static("x-goog-api-key"),
            key: apikey.to_owned(),
        }),
    }
}
