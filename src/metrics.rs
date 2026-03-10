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
    let user = request_metadata.user_identity.clone().unwrap_or_default();
    let labels = [
        ("provider", request_metadata.provider_key.clone()),
        ("user", user),
    ];
    metrics::counter!("lacuna_provider_requests_total", &labels).increment(1);
}

pub fn record_response(request_metadata: &RequestMetadata, response_metadata: &ResponseMetadata) {
    let user = request_metadata.user_identity.clone().unwrap_or_default();
    let labels = [
        ("provider", request_metadata.provider_key.clone()),
        ("handler", request_metadata.api_handler_id.clone()),
        ("user", user),
    ];
    if let Some(tokens) = response_metadata.input_tokens {
        metrics::counter!("lacuna_provider_input_tokens_total", &labels).increment(tokens);
    }
    if let Some(tokens) = response_metadata.output_tokens {
        metrics::counter!("lacuna_provider_output_tokens_total", &labels).increment(tokens);
    }
}
