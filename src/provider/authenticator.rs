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

#[derive(Debug)]
pub(super) enum Authenticator {
    None,
    Bearer {
        key: String,
    },
    ApiKey {
        header: reqwest::header::HeaderName,
        key: String,
    },
}

impl Authenticator {
    pub(super) async fn authenticate(
        &self,
        request: &mut reqwest::Request,
    ) -> Result<(), AuthenticatorError> {
        match self {
            Self::None => Ok(()),

            Self::Bearer { key } => {
                let value = format!("Bearer {key}")
                    .parse()
                    .map_err(AuthenticatorError::InvalidBearerToken)?;
                request
                    .headers_mut()
                    .insert(reqwest::header::AUTHORIZATION, value);
                Ok(())
            }

            Self::ApiKey { header, key } => {
                let value = key
                    .parse()
                    .map_err(|source| AuthenticatorError::InvalidApiKey {
                        header: header.clone(),
                        source,
                    })?;
                request.headers_mut().insert(header.clone(), value);
                Ok(())
            }
        }
    }
}

pub(super) fn build_authenticator(authorization: Option<&config::Authorization>) -> Authenticator {
    match authorization {
        None => Authenticator::None,
        Some(config::Authorization::Bearer { apikey }) => Authenticator::Bearer {
            key: apikey.clone(),
        },
        Some(config::Authorization::XApiKey { apikey }) => Authenticator::ApiKey {
            header: reqwest::header::HeaderName::from_static("x-api-key"),
            key: apikey.clone(),
        },
        Some(config::Authorization::XGoogApiKey { apikey }) => Authenticator::ApiKey {
            header: reqwest::header::HeaderName::from_static("x-goog-api-key"),
            key: apikey.clone(),
        },
    }
}
