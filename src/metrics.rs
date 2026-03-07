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

pub fn record_request(provider: &str, user: &str) {
    let labels = [("provider", provider.to_owned()), ("user", user.to_owned())];
    metrics::counter!("lacuna_provider_requests_total", &labels).increment(1);
}
