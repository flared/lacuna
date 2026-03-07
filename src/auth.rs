use axum::{extract::Request, http, middleware::Next, response::Response};

#[derive(Clone, Debug)]
pub struct CallerIdentity {
    pub email: String,
}

pub async fn identity_middleware(
    header_name: String,
    mut request: Request,
    next: Next,
) -> Response {
    let caller = request
        .headers()
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned());

    if let Some(ref email) = caller {
        request.extensions_mut().insert(CallerIdentity {
            email: email.clone(),
        });
    }

    next.run(request).await
}

pub fn get_caller_identity(request: &http::Request<impl std::any::Any>) -> Option<String> {
    request
        .extensions()
        .get::<CallerIdentity>()
        .map(|id| id.email.clone())
}
