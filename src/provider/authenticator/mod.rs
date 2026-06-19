pub(crate) mod iam;

use super::compatibility::Compatibility;
use crate::config;
use iam::IamRequestSigner;

pub(super) const MAX_BUFFERED_BODY_BYTES: usize = 25_000_000;

/// How the outbound body must be handled before `authenticate` runs.
#[derive(Debug)]
pub(super) enum BodyMode {
    Stream,
    Buffer { max_bytes: usize },
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
    Iam(IamRequestSigner),
}

impl Authenticator {
    pub(super) async fn authenticate(
        &self,
        request: &mut reqwest::Request,
    ) -> Result<(), anyhow::Error> {
        match self {
            Self::None => Ok(()),

            Self::Bearer { key } => {
                let value = format!("Bearer {key}")
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid bearer token: {e}"))?;
                request
                    .headers_mut()
                    .insert(reqwest::header::AUTHORIZATION, value);
                Ok(())
            }

            Self::ApiKey { header, key } => {
                let value = key
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid api key for header {header}: {e}"))?;
                request.headers_mut().insert(header.clone(), value);
                Ok(())
            }

            Self::Iam(signer) => signer.sign(request).await,
        }
    }

    pub(super) fn body_mode(&self) -> BodyMode {
        match self {
            Self::None | Self::Bearer { .. } | Self::ApiKey { .. } => BodyMode::Stream,
            Self::Iam(_) => BodyMode::Buffer {
                max_bytes: MAX_BUFFERED_BODY_BYTES,
            },
        }
    }

    // Surface failures on launch instead of first request
    pub(super) async fn preflight(&self) -> Result<(), anyhow::Error> {
        match self {
            Self::None | Self::Bearer { .. } | Self::ApiKey { .. } => Ok(()),
            Self::Iam(signer) => signer.resolve_credentials().await,
        }
    }

    pub(super) fn strip_auth_headers(&self, headers: &mut http::HeaderMap) {
        match self {
            Self::None | Self::Bearer { .. } | Self::ApiKey { .. } => {}
            Self::Iam(signer) => signer.strip_auth_headers(headers),
        }
    }
}

pub(super) async fn build_authenticator(
    authorization: Option<&config::Authorization>,
    compatibility: &Compatibility,
) -> Result<Authenticator, anyhow::Error> {
    Ok(match authorization {
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
        Some(config::Authorization::Iam {}) => {
            Authenticator::Iam(IamRequestSigner::new(compatibility).await?)
        }
    })
}
