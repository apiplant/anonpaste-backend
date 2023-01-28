use anonpaste::{
    mailer::ReportMessage,
    models::report::{CreateReport, Report},
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
async fn fetch_reports() {
    let config = get_test_config();
    let (router, app_state) = get_app(&config).await.unwrap();
    let mut conn = app_state.pool.acquire().await.unwrap();
    Report::create(
        &mut conn,
        &app_state.mailer,
        CreateReport {
            links: [
                "https://website/test-id#magic-key".to_string(),
                "https://website/test-id-2#magic-key-2".to_string(),
            ]
            .to_vec(),
            message: "Hey, this is my client's content, please remove it".to_string(),
            email: "federico@leaksdown.apiplant.com".to_string(),
        },
    )
    .await
    .unwrap();

    let response = router
        .with_state(app_state)
        .oneshot(
            Request::builder()
                .uri("/api/report")
                .header("x-real-ip", "127.0.0.1")
                .header("content-type", "application/json")
                .header("Authorization", format!("Bearer {}", &config.admin_token))
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
        json!([{"email": "federico@leaksdown.apiplant.com", "links": ["https://website/test-id#magic-key", "https://website/test-id-2#magic-key-2"], "message": "Hey, this is my client's content, please remove it"}])
    );
}

#[tokio::test]
async fn create_report() {
    let config = get_test_config();
    let (router, app_state) = get_app(&config).await.unwrap();
    let mut conn = app_state.pool.acquire().await.unwrap();

    let app = router.with_state(app_state.clone());

    let paste_payload = CreateReport {
        links: [
            "https://website/test-id#magic-key".to_string(),
            "https://website/test-id-2#magic-key-2".to_string(),
        ]
        .to_vec(),
        message: "Hey, this is my client's content, please remove it".to_string(),
        email: "federico@leaksdown.apiplant.com".to_string(),
    };
    let body = serde_json::to_string(&paste_payload).unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/report")
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

    let reports = Report::list(&mut conn).await.unwrap();
    let report = reports.first().unwrap();

    assert_eq!(
        *report,
        Report {
            links: [
                "https://website/test-id#magic-key".to_string(),
                "https://website/test-id-2#magic-key-2".to_string(),
            ]
            .to_vec(),
            message: "Hey, this is my client's content, please remove it".to_string(),
            email: "federico@leaksdown.apiplant.com".to_string(),
        }
    );

    let sent = app_state.mailer.get_sent_emails();

    assert_eq!(
        sent,
        vec![ReportMessage {
            email_from: "test@test.com".to_string(), 
            email_name: "test test".to_string(), 
            to: "federico@leaksdown.apiplant.com".to_string(), 
            content: "Thanks for reaching out and initiating our DMCA Report procedure.\nYou reported the following links:\n\nhttps://website/test-id#magic-key\nhttps://website/test-id-2#magic-key-2\n\nAn operator will get back to you within 24hrs.\n\nKind Regards,\nAnonPaste Team".to_string(), 
            subject: "DMCA Report Initiated".to_string() 
        }, ReportMessage {
            email_from: "test@test.com".to_string(), 
            email_name: "test test".to_string(), 
            to: "test@test.com".to_string(), 
            content: "These links have been reported by federico@leaksdown.apiplant.com:\n\nhttps://website/test-id#magic-key\nhttps://website/test-id-2#magic-key-2\n\nAnonPaste Team".to_string(), 
            subject: "DMCA Report Initiated".to_string() 
        }]
    );
}
