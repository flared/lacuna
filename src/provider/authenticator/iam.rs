use std::str::FromStr;
use std::time::{Duration, SystemTime};

use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_credential_types::provider::{ProvideCredentials, SharedCredentialsProvider};
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use aws_smithy_runtime_api::client::identity::Identity;

use anyhow::Context;

use crate::provider::compatibility::Compatibility;

/// Service name used in SigV4 signing process derived from compatibility flags
fn aws_signing_services(compatibility: &Compatibility) -> Vec<&'static str> {
    let mut services = Vec::new();
    if compatibility.bedrock_model_invoke {
        services.push("bedrock");
    }
    // Future :
    // if compatibility.anthropic_messages { services.push("aws-external-anthropic"); }
    services
}
/// Refresh when within this buffer of expiry.
/// (The SDK's identity cache uses 10s + jitter; 120s is conservative.)
const EXPIRY_BUFFER: Duration = Duration::from_secs(120);

/// Expiry-aware cache over the AWS default credential chain.
/// The default chain does not cache when called directly
#[derive(Debug)]
struct CachedAwsCredentials {
    credential_provider: SharedCredentialsProvider,
    credentials: tokio::sync::Mutex<Option<Credentials>>,
}

impl CachedAwsCredentials {
    fn new(credential_provider: SharedCredentialsProvider) -> Self {
        Self {
            credential_provider,
            credentials: tokio::sync::Mutex::new(None),
        }
    }

    async fn get(&self) -> Result<Credentials, anyhow::Error> {
        let mut guard = self.credentials.lock().await;
        if let Some(creds) = guard.as_ref() {
            let still_fresh = match creds.expiry() {
                None => true, // static keys: cache for the process lifetime
                Some(exp) => SystemTime::now() + EXPIRY_BUFFER < exp,
            };
            if still_fresh {
                return Ok(creds.clone());
            }
        }
        match self.credential_provider.provide_credentials().await {
            Ok(fresh) => {
                *guard = Some(fresh.clone());
                Ok(fresh)
            }
            Err(e) => Err(anyhow::anyhow!("failed to resolve AWS credentials: {e}")),
        }
    }
}

#[derive(Debug)]
pub(in crate::provider) struct IamRequestSigner {
    region: String,
    service: &'static str,
    credentials: CachedAwsCredentials,
}

impl IamRequestSigner {
    pub(super) async fn new(compatibility: &Compatibility) -> Result<Self, anyhow::Error> {
        let service = match aws_signing_services(compatibility).as_slice() {
            [service] => *service,
            [] => anyhow::bail!(
                "`iam` authorization requires an AWS-backed compatibility flag such as \
                 `bedrock_model_invoke`"
            ),
            services => anyhow::bail!(
                "`iam` authorization is ambiguous across AWS signing services {services:?}; \
                 use a separate provider per service"
            ),
        };
        // The signing region is resolved entirely by the AWS SDK
        // (AWS_REGION env variable, AWS profile, etc.).
        let sdk_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let region = sdk_config
            .region()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "`iam` authorization requires a signing region; set the AWS_REGION \
                     environment variable or a `region` in your AWS profile"
                )
            })?
            .to_string();
        let credentials_provider = sdk_config
            .credentials_provider()
            .expect("the default chain should have a credentials provider");
        Ok(Self {
            region,
            service,
            credentials: CachedAwsCredentials::new(credentials_provider),
        })
    }

    /// Force resolving credentials
    pub(super) async fn resolve_credentials(&self) -> Result<(), anyhow::Error> {
        self.credentials.get().await.map(|_| ())
    }

    /// SigV4-sign the outbound request.
    /// nothing may mutate the request after this step except additions that stay unsigned.
    pub(super) async fn sign(&self, request: &mut reqwest::Request) -> Result<(), anyhow::Error> {
        self.sign_at(request, SystemTime::now()).await
    }

    async fn sign_at(
        &self,
        request: &mut reqwest::Request,
        time: SystemTime,
    ) -> Result<(), anyhow::Error> {
        let creds = self.credentials.get().await?;
        let identity: Identity = creds.into();

        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(&self.region)
            .name(self.service)
            .time(time)
            .settings(SigningSettings::default())
            .build()
            .context("request signing failed")?
            .into();

        let body_bytes = request
            .body()
            .and_then(|body| body.as_bytes())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "request signing failed: request body must be buffered for SigV4 signing"
                )
            })?;

        // `content-type` is the only client provided header we sign.
        // reqwest/hyper finalize the others (content-length, accept-encoding, framing)
        // after this step, signing them would risk an intermittent SignatureDoesNotMatch.
        let mut extra_headers_to_sign: Vec<(&str, &str)> = Vec::new();
        if let Some(value) = request.headers().get("content-type") {
            let value = value.to_str().map_err(|e| {
                anyhow::anyhow!(
                    "request signing failed: header content-type has a non-ASCII value: {e}"
                )
            })?;
            extra_headers_to_sign.push(("content-type", value));
        }

        let signable_request = SignableRequest::new(
            request.method().as_str(),
            request.url().as_str(),
            extra_headers_to_sign.into_iter(),
            SignableBody::Bytes(body_bytes),
        )
        .context("request signing failed")?;

        let (instructions, _signature) = sign(signable_request, &signing_params)
            .context("request signing failed")?
            .into_parts();

        // aws-sdk `SigningInstructions::apply_to_request_http1x` expect a `http::Request`.
        // We therefore manually apply the signed headers to our `reqwest::Request`
        let (signed_headers, _params) = instructions.into_parts();
        for header in signed_headers {
            let name = http::HeaderName::from_str(header.name()).map_err(|e| {
                anyhow::anyhow!("request signing failed: invalid signed header name: {e}")
            })?;
            let mut value = http::HeaderValue::from_str(header.value()).map_err(|e| {
                anyhow::anyhow!(
                    "request signing failed: invalid signed header value for {name}: {e}"
                )
            })?;
            value.set_sensitive(header.sensitive());
            request.headers_mut().insert(name, value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use aws_credential_types::provider::error::CredentialsError;
    use aws_credential_types::provider::future;

    /// Counts every chain walk and returns one response per call
    #[derive(Debug)]
    struct FakeCredentialProvider {
        call_count: Arc<AtomicUsize>,
        responses: Vec<Option<Credentials>>,
    }

    impl ProvideCredentials for FakeCredentialProvider {
        fn provide_credentials<'a>(&'a self) -> future::ProvideCredentials<'a>
        where
            Self: 'a,
        {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst);
            let response = self
                .responses
                .get(n.min(self.responses.len().saturating_sub(1)))
                .and_then(Option::as_ref);
            future::ProvideCredentials::ready(match response {
                Some(creds) => Ok(creds.clone()),
                None => Err(CredentialsError::not_loaded("fake provider failure")),
            })
        }
    }

    fn fake_credential_provider(
        responses: Vec<Option<Credentials>>,
    ) -> (SharedCredentialsProvider, Arc<AtomicUsize>) {
        let call_count = Arc::new(AtomicUsize::new(0));
        let provider = SharedCredentialsProvider::new(FakeCredentialProvider {
            call_count: call_count.clone(),
            responses,
        });
        (provider, call_count)
    }

    fn creds_with_expiry(expiry: Option<SystemTime>) -> Credentials {
        Credentials::new("AKIATESTKEY", "test-secret", None, expiry, "fake")
    }

    #[tokio::test]
    async fn credentials_not_fetched_again_while_cache_is_fresh() {
        let expiry = SystemTime::now() + Duration::from_secs(3600);
        let (provider, call_count) =
            fake_credential_provider(vec![Some(creds_with_expiry(Some(expiry)))]);
        let cache = CachedAwsCredentials::new(provider);

        for _ in 0..5 {
            let creds = cache.get().await.unwrap();
            assert_eq!(creds.access_key_id(), "AKIATESTKEY");
        }
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn credentials_cached_with_static_keys_never_re_resolve() {
        let (provider, call_count) = fake_credential_provider(vec![Some(creds_with_expiry(None))]);
        let cache = CachedAwsCredentials::new(provider);

        for _ in 0..5 {
            cache.get().await.unwrap();
        }
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn credentials_re_resolve_when_inside_the_expiry_buffer() {
        // Credentials expiring sooner than EXPIRY_BUFFER are not fresh
        let expiry = SystemTime::now() + Duration::from_secs(60);
        assert!(expiry < SystemTime::now() + EXPIRY_BUFFER);
        let (provider, call_count) =
            fake_credential_provider(vec![Some(creds_with_expiry(Some(expiry)))]);
        let cache = CachedAwsCredentials::new(provider);

        for _ in 0..3 {
            cache.get().await.unwrap();
        }
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn expired_credentials_are_refetched_and_replace_the_cache() {
        // First resolution hands back an already-expired credential; the second
        // a fresh one valid for an hour.
        let expired = Credentials::new(
            "OLDKEY",
            "old-secret",
            None,
            Some(SystemTime::now() - Duration::from_secs(1)),
            "fake",
        );
        let fresh = Credentials::new(
            "NEWKEY",
            "new-secret",
            None,
            Some(SystemTime::now() + Duration::from_secs(3600)),
            "fake",
        );
        let (provider, call_count) = fake_credential_provider(vec![Some(expired), Some(fresh)]);
        let cache = CachedAwsCredentials::new(provider);

        // First call caches the (already expired) credential and serves it.
        assert_eq!(cache.get().await.unwrap().access_key_id(), "OLDKEY");
        // Because it is expired, the next call must re-resolve and adopt the new one.
        assert_eq!(cache.get().await.unwrap().access_key_id(), "NEWKEY");
        // The fresh credential is now cached and served without re-resolving.
        assert_eq!(cache.get().await.unwrap().access_key_id(), "NEWKEY");
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn preflight_fails_fast_when_credentials_cannot_resolve() {
        // `resolve_credentials` backs the startup preflight check: a provider
        // that can never authenticate must fail the launch instead of failing
        // every later request.
        let (provider, _calls) = fake_credential_provider(vec![]);
        let signer = IamRequestSigner {
            region: "us-east-1".to_owned(),
            service: "bedrock",
            credentials: CachedAwsCredentials::new(provider),
        };

        let err = signer
            .resolve_credentials()
            .await
            .expect_err("preflight should fail when credentials cannot resolve");
        assert!(
            err.to_string()
                .contains("failed to resolve AWS credentials"),
            "unexpected error: {err:#}"
        );
    }

    fn test_signer(credentials: Credentials) -> IamRequestSigner {
        IamRequestSigner {
            region: "us-east-1".to_owned(),
            service: "bedrock",
            credentials: CachedAwsCredentials::new(SharedCredentialsProvider::new(credentials)),
        }
    }

    fn build_request(headers: &[(&str, &str)]) -> reqwest::Request {
        let mut builder = reqwest::Client::new()
            .post("https://bedrock-runtime.us-east-1.amazonaws.com/model/test-model/invoke")
            .header("content-type", "application/json");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder
            .body(r#"{"prompt":"hello"}"#)
            .build()
            .expect("test request should build")
    }

    fn signed_headers(request: &reqwest::Request) -> String {
        let authorization_header = request
            .headers()
            .get("authorization")
            .expect("request should be signed")
            .to_str()
            .unwrap();

        assert!(authorization_header.starts_with("AWS4-HMAC-SHA256 "));
        authorization_header
            .split("SignedHeaders=")
            .nth(1)
            .expect("authorization header should list signed headers")
            .split(',')
            .next()
            .unwrap()
            .to_owned()
    }

    #[tokio::test]
    async fn non_sigv4_headers_are_kept_unsigned() {
        let signer = test_signer(creds_with_expiry(None));
        let mut request = build_request(&[("anthropic-version", "2023-06-01")]);

        signer.sign(&mut request).await.unwrap();

        assert_eq!(
            request.headers().get("anthropic-version").unwrap(),
            "2023-06-01"
        );
        assert_eq!(signed_headers(&request), "content-type;host;x-amz-date");
    }

    #[tokio::test]
    async fn sign_leaves_body_unchanged() {
        let signer = test_signer(creds_with_expiry(None));
        let mut request = build_request(&[]);

        signer.sign(&mut request).await.unwrap();

        assert_eq!(
            request.body().and_then(|b| b.as_bytes()).unwrap(),
            br#"{"prompt":"hello"}"#
        );
    }
}
