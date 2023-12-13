use anonpaste::server::{get_app, get_test_config};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn health_check() {
    let (router, app_state) = get_app(&get_test_config()).await.unwrap();

    let response = router
        .with_state(app_state)
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&body[..], b"ok");
}
