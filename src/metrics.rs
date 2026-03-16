use crate::http_middleware::auth::Identity;
use crate::request_metadata::{RequestMetadata, ResponseMetadata};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::LazyLock;

static PROMETHEUS_HANDLE: LazyLock<PrometheusHandle> = LazyLock::new(|| {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder")
});

pub fn init() {
    LazyLock::force(&PROMETHEUS_HANDLE);
}

pub fn render() -> String {
    PROMETHEUS_HANDLE.render()
}

pub fn record_request(request_metadata: &RequestMetadata) {
    let user = match &request_metadata.user_identity {
        Some(Identity::LoginUser(email)) => email.clone(),
        _ => String::new(),
    };
    let model = request_metadata
        .inspected
        .as_ref()
        .and_then(|m| m.model.clone())
        .unwrap_or_default();

    let labels = [
        ("provider", request_metadata.provider_key.clone()),
        ("user", user),
        ("model", model),
    ];
    metrics::counter!("lacuna_provider_requests_total", &labels).increment(1);
}

pub fn record_response(request_metadata: &RequestMetadata, response_metadata: &ResponseMetadata) {
    let user = match &request_metadata.user_identity {
        Some(Identity::LoginUser(email)) => email.clone(),
        _ => String::new(),
    };
    let model = request_metadata
        .inspected
        .as_ref()
        .and_then(|m| m.model.clone())
        .unwrap_or_default();
    let labels = [
        ("provider", request_metadata.provider_key.clone()),
        ("handler", request_metadata.api_handler_id.clone()),
        ("user", user),
        ("model", model),
    ];
    if let Some(tokens) = response_metadata.input_tokens {
        metrics::counter!("lacuna_provider_tokens_input_total", &labels).increment(tokens);
    }
    if let Some(tokens) = response_metadata.output_tokens {
        metrics::counter!("lacuna_provider_tokens_output_total", &labels).increment(tokens);
    }
    let total_tokens =
        response_metadata.input_tokens.unwrap_or(0) + response_metadata.output_tokens.unwrap_or(0);
    if total_tokens > 0 {
        metrics::counter!("lacuna_provider_tokens_total", &labels).increment(total_tokens);
    }
}
