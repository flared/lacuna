use axum::{extract::Request, extract::State, http, middleware::Next, response::Response};
use std::sync::Arc;

use crate::user_agent::{UserAgentExtractor, UserAgentMetadata};

pub async fn user_agent_middleware(
    State(extractor): State<Arc<UserAgentExtractor>>,
    mut request: Request,
    next: Next,
) -> Response {
    let metadata = request
        .headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|raw| extractor.extract(raw));

    if let Some(metadata) = metadata {
        request.extensions_mut().insert(metadata);
    }

    next.run(request).await
}

pub fn get_user_agent(request: &http::Request<impl std::any::Any>) -> Option<UserAgentMetadata> {
    request.extensions().get::<UserAgentMetadata>().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, middleware, routing::get};
    use http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::user_agent::UserAgentExtractor;

    /// Build a minimal router that runs the user_agent middleware and echoes back
    /// the extracted metadata as the response body.
    fn test_router() -> Router {
        let extractor = Arc::new(UserAgentExtractor::new(vec![]));
        Router::new()
            .route("/", get(handler))
            .layer(middleware::from_fn_with_state(
                extractor,
                user_agent_middleware,
            ))
    }

    async fn handler(request: Request<Body>) -> String {
        match get_user_agent(&request) {
            Some(meta) => format!("{}|{}", meta.normalized, meta.raw),
            None => "none".to_owned(),
        }
    }

    async fn send_request(user_agent: Option<&str>) -> String {
        let app = test_router();
        let mut builder = Request::builder().uri("/");
        if let Some(ua) = user_agent {
            builder = builder.header("user-agent", ua);
        }
        let response = app
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn extracts_known_user_agent() {
        let body = send_request(Some("claude-code/1.0")).await;
        assert_eq!(body, "claude-code|claude-code/1.0");
    }

    #[tokio::test]
    async fn unknown_user_agent() {
        let body = send_request(Some("Mozilla/5.0")).await;
        assert_eq!(body, "unknown|Mozilla/5.0");
    }

    #[tokio::test]
    async fn missing_user_agent_header() {
        let body = send_request(None).await;
        assert_eq!(body, "none");
    }
}
