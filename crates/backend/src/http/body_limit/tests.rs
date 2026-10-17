//! Проверки глобального предела HTTP request body и локальных ограничений.

use axum::{
    Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    http::{Method, StatusCode},
    middleware,
    routing::{get, post},
};

use super::MAX_REQUEST_BODY_BYTES;

async fn request(method: Method, path: &str, payload: Vec<u8>) -> (StatusCode, String) {
    let app = Router::new()
        .route(
            "/api/upload",
            post(|body: Bytes| async move { body.len().to_string() })
                .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES)),
        )
        .route(
            "/api/regular",
            post(|body: Bytes| async move { body.len().to_string() }),
        )
        .route("/health", get(|| async { "ok" }))
        .layer(super::layer())
        .layer(middleware::from_fn(super::log_rejection));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .request(method, format!("http://{address}{path}"))
        .body(payload)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    server.abort();
    (status, body)
}

#[tokio::test]
async fn exact_nine_mib_body_is_accepted() {
    let (status, body) = request(
        Method::POST,
        "/api/upload",
        vec![b'a'; MAX_REQUEST_BODY_BYTES],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, MAX_REQUEST_BODY_BYTES.to_string());
}

#[tokio::test]
async fn oversized_content_length_is_rejected_with_413() {
    let (status, _) = request(
        Method::POST,
        "/api/upload",
        vec![b'a'; MAX_REQUEST_BODY_BYTES + 1],
    )
    .await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn default_axum_limit_remains_stricter_and_health_works() {
    let (status, _) = request(
        Method::POST,
        "/api/regular",
        vec![b'a'; 2 * 1024 * 1024 + 1],
    )
    .await;
    let (health_status, health_body) = request(Method::GET, "/health", Vec::new()).await;

    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(health_status, StatusCode::OK);
    assert_eq!(health_body, "ok");
}
