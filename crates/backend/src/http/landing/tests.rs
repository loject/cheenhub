//! Контракт публичного лендинга на HTTP-границе.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{Router, http::StatusCode, routing::get};

use super::{LANDING_CSP, mount_landing};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cheenhub-landing-test-{}", uuid::Uuid::new_v4()));
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

async fn request(
    directory: &Path,
    path: &str,
    head: bool,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let router = mount_landing(
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
    let url = format!("http://{address}{path}");
    let response = if head {
        client.head(url).send().await.unwrap()
    } else {
        client.get(url).send().await.unwrap()
    };
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap();
    server.abort();
    (status, headers, body)
}

#[tokio::test]
async fn root_serves_landing_with_strict_csp() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("landing.html"), "<h1>CheenHub</h1>").unwrap();

    let (status, headers, body) = request(fixture.path(), "/", false).await;

    assert_eq!(status, StatusCode::OK);
    for (name, expected) in [
        ("content-type", "text/html; charset=utf-8"),
        ("cache-control", "no-cache"),
        ("content-security-policy", LANDING_CSP),
        ("x-content-type-options", "nosniff"),
        ("x-frame-options", "DENY"),
        ("referrer-policy", "strict-origin-when-cross-origin"),
    ] {
        assert_eq!(
            headers.get(name).unwrap().to_str().unwrap(),
            expected,
            "{name}"
        );
    }
    assert_eq!(body, "<h1>CheenHub</h1>");
}

#[tokio::test]
async fn missing_landing_keeps_original_fallback() {
    let fixture = Fixture::new();

    let (status, _, _) = request(fixture.path(), "/", false).await;

    assert_eq!(status, StatusCode::IM_A_TEAPOT);
}

#[tokio::test]
async fn landing_does_not_intercept_api() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("landing.html"), "landing").unwrap();

    let (status, _, body) = request(fixture.path(), "/api/ping", false).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "pong");
}

#[tokio::test]
async fn head_returns_landing_headers_without_body() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("landing.html"), "landing").unwrap();

    let (status, headers, body) = request(fixture.path(), "/", true).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-security-policy").unwrap(), LANDING_CSP);
    assert!(body.is_empty());
}
