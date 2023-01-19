use anonpaste::{
    models::paste::{CreatePaste, Paste},
    server::{get_app, get_test_config},
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{
    json,
    Value::{self, Null},
};
use tower::ServiceExt;

#[tokio::test]
async fn fetch_paste() {
    let (router, app_state) = get_app(&get_test_config()).await.unwrap();
    let mut conn = app_state.pool.acquire().await.unwrap();
    Paste::create(
        &mut conn,
        CreatePaste {
            id: "test-id".to_string(),
            content: "Hello".to_string(),
            expiry_time: None,
            expiry_views: None,
        },
    )
    .await
    .unwrap();

    let response = router
        .with_state(app_state)
        .oneshot(
            Request::builder()
                .uri("/api/paste/test-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({"content": "Hello".to_string(), "id": "test-id".to_string(), "expiryTime": Null, "expiryViews": Null} )
    );
}

#[tokio::test]
async fn fetch_paste_not_found() {
    let (router, app_state) = get_app(&get_test_config()).await.unwrap();
    let response = router
        .with_state(app_state)
        .oneshot(
            Request::builder()
                .uri("/api/paste/test-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn fetch_paste_expiry_views() {
    let (router, app_state) = get_app(&get_test_config()).await.unwrap();
    let mut conn = app_state.pool.acquire().await.unwrap();
    Paste::create(
        &mut conn,
        CreatePaste {
            id: "test-id".to_string(),
            content: "Hello".to_string(),
            expiry_time: None,
            expiry_views: Some(1),
        },
    )
    .await
    .unwrap();

    let app = router.with_state(app_state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/paste/test-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        body,
        json!({"content": "Hello".to_string(), "id": "test-id".to_string(), "expiryTime": Null, "expiryViews": 1} )
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/paste/test-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn create_paste() {
    let config = get_test_config();
    let (router, app_state) = get_app(&config).await.unwrap();
    let mut conn = app_state.pool.acquire().await.unwrap();

    let app = router.with_state(app_state);

    let paste_payload = CreatePaste {
        id: "test-id".to_string(),
        content: "Wow".to_string(),
        expiry_views: None,
        expiry_time: None,
    };
    let body = serde_json::to_string(&paste_payload).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/paste")
                .header("x-real-ip", "127.0.0.1")
                .header("content-type", "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    println!("{:?}", body);
    let body: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body, json!(Null));

    let paste = Paste::view(&mut conn, "test-id".to_string()).await.unwrap();

    assert_eq!(
        paste,
        Paste {
            content: "Wow".to_string(),
            id: "test-id".to_string(),
            expiry_time: None,
            expiry_views: None
        }
    )
}
