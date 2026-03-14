use axum::{extract::Request, http, middleware::Next, response::Response};

#[derive(Clone, Debug)]
pub enum Identity {
    AnonUser,
    LoginUser(String),
}

pub async fn identity_middleware(
    header_name: String,
    mut request: Request,
    next: Next,
) -> Response {
    let identity = match request
        .headers()
        .get(&header_name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
    {
        Some(email) => Identity::LoginUser(email),
        None => Identity::AnonUser,
    };

    request.extensions_mut().insert(identity);

    next.run(request).await
}

pub fn get_caller_identity(request: &http::Request<impl std::any::Any>) -> Option<Identity> {
    request.extensions().get::<Identity>().cloned()
}
