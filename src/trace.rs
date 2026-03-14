use axum::body::Body;
use axum::http::{Request, Response};
use tower_http::classify::ServerErrorsAsFailures;
use tower_http::trace::{
    DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest, MakeSpan, OnResponse,
    TraceLayer,
};
use tracing::{Span, info};

use crate::http_middleware::auth;

#[derive(Debug, Clone)]
pub struct LacunaMakeSpan;

impl MakeSpan<Body> for LacunaMakeSpan {
    fn make_span(&mut self, request: &Request<Body>) -> Span {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();
        let caller = match auth::get_caller_identity(request) {
            Some(auth::Identity::LoginUser(email)) => email,
            _ => "-".to_string(),
        };
        tracing::info_span!("request", %method, %path, %caller)
    }
}

#[derive(Debug, Clone)]
pub struct LacunaOnResponse;

impl OnResponse<Body> for LacunaOnResponse {
    fn on_response(self, response: &Response<Body>, latency: std::time::Duration, _span: &Span) {
        let status = response.status().as_u16();
        info!(status, latency_ms = latency.as_millis(), "response");
    }
}

pub type Layer = TraceLayer<
    tower_http::classify::SharedClassifier<ServerErrorsAsFailures>,
    LacunaMakeSpan,
    DefaultOnRequest,
    LacunaOnResponse,
    DefaultOnBodyChunk,
    DefaultOnEos,
    DefaultOnFailure,
>;

pub fn layer() -> Layer {
    TraceLayer::new_for_http()
        .make_span_with(LacunaMakeSpan)
        .on_response(LacunaOnResponse)
}
