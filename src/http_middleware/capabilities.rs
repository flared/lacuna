use axum::{extract::Request, http, middleware::Next, response::Response};
use tracing::warn;

use crate::capabilities::{Capabilities, parse_capabilities};

fn capabilities_from_request(header_name: &str, request: &Request) -> Capabilities {
    let header_value = request
        .headers()
        .get(header_name)
        .and_then(|v| v.to_str().ok());
    match header_value {
        Some(value) => match parse_capabilities(value) {
            Ok(capabilities) => capabilities,
            Err(e) => {
                warn!("failed to parse capabilities header: {e}");
                Capabilities::deny_all()
            }
        },
        None => Capabilities::deny_all(),
    }
}

pub async fn capabilities_middleware(
    header_name: String,
    mut request: Request,
    next: Next,
) -> Response {
    let capabilities = capabilities_from_request(&header_name, &request);
    request.extensions_mut().insert(capabilities);
    next.run(request).await
}

pub fn get_capabilities(request: &http::Request<impl std::any::Any>) -> Option<Capabilities> {
    request.extensions().get::<Capabilities>().cloned()
}
