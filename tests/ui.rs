use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn build_app_with_test_assets() -> axum::Router {
    let dir = std::env::temp_dir().join("lacuna_test_ui");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("index.html"), "<html><body>test</body></html>").unwrap();
    lacuna::app::AppBuilder::new().assets_path(&dir).build()
}

#[tokio::test]
async fn ui_index() {
    let response = build_app_with_test_assets()
        .oneshot(Request::builder().uri("/ui/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("<html"));
}

#[tokio::test]
async fn root_redirects_to_ui() {
    let response = build_app_with_test_assets()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
    assert_eq!(response.headers().get("location").unwrap(), "/ui/");
}
