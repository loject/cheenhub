//! Проверки HTTP-контракта файла service worker.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::get,
};

use super::mount_service_worker;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cheenhub-service-worker-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn request(directory: &Path, path: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let router = mount_service_worker(
        Router::<()>::new()
            .route("/api/ping", get(|| async { "pong" }))
            .fallback(|| async { StatusCode::IM_A_TEAPOT }),
        directory,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .get(format!("http://{address}{path}"))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.unwrap().to_vec();
    server.abort();
    (status, headers, body)
}

#[tokio::test]
async fn existing_service_worker_has_expected_headers_and_body() {
    let fixture = Fixture::new();
    fs::write(
        fixture.path().join("sw.js"),
        "self.addEventListener('fetch', () => {});",
    )
    .unwrap();

    let (status, headers, body) = request(fixture.path(), "/sw.js").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_slice(),
        b"self.addEventListener('fetch', () => {});"
    );
    for (name, value) in [
        ("content-type", "application/javascript; charset=utf-8"),
        ("cache-control", "no-cache, no-store, must-revalidate"),
        ("service-worker-allowed", "/"),
        ("x-content-type-options", "nosniff"),
    ] {
        assert_eq!(
            headers.get(name).unwrap().to_str().unwrap(),
            value,
            "{name}"
        );
    }
}

#[tokio::test]
async fn missing_service_worker_keeps_previous_fallback() {
    let fixture = Fixture::new();

    let (status, _, _) = request(fixture.path(), "/sw.js").await;

    assert_eq!(status, StatusCode::IM_A_TEAPOT);
}

#[tokio::test]
async fn service_worker_does_not_intercept_api() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("sw.js"), "/* sw */").unwrap();

    let (status, _, body) = request(fixture.path(), "/api/ping").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_slice(), b"pong");
}
