//! Проверки точной доставки ресурсов PWA.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::get,
};

use super::{OFFLINE_CSP, mount_pwa_files};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cheenhub-pwa-files-test-{}", uuid::Uuid::new_v4()));
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

async fn request(directory: &Path, path: &str, head: bool) -> (StatusCode, HeaderMap, Vec<u8>) {
    let app = mount_pwa_files(
        Router::<()>::new()
            .route("/api/ping", get(|| async { "pong" }))
            .fallback(|| async { StatusCode::IM_A_TEAPOT }),
        directory,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
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
    let body = response.bytes().await.unwrap().to_vec();
    server.abort();
    (status, headers, body)
}

#[tokio::test]
async fn offline_html_has_restricted_csp_and_correct_mime_type() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("offline.html"), "<h1>Offline</h1>").unwrap();

    let (status, headers, body) = request(fixture.path(), "/offline.html", false).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap(),
        "text/html; charset=utf-8"
    );
    assert_eq!(headers.get("content-security-policy").unwrap(), OFFLINE_CSP);
    assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(headers.get("cache-control").unwrap(), "no-cache");
    assert_eq!(body.as_slice(), b"<h1>Offline</h1>");
}

#[tokio::test]
async fn registration_script_has_javascript_mime_type() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("pwa-register.js"), "console.log('pwa')").unwrap();

    let (status, headers, body) = request(fixture.path(), "/pwa-register.js", false).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap(),
        "application/javascript; charset=utf-8"
    );
    assert_eq!(headers.get("cache-control").unwrap(), "no-cache");
    assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(body.as_slice(), b"console.log('pwa')");
}

#[tokio::test]
async fn missing_files_preserve_existing_fallback() {
    let fixture = Fixture::new();

    for path in ["/offline.html", "/pwa-register.js"] {
        let (status, _, _) = request(fixture.path(), path, false).await;
        assert_eq!(status, StatusCode::IM_A_TEAPOT, "{path}");
    }
}

#[tokio::test]
async fn head_does_not_send_html_body_and_api_remains_unchanged() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("offline.html"), "offline").unwrap();

    let (status, headers, body) = request(fixture.path(), "/offline.html", true).await;
    let (api_status, _, api_body) = request(fixture.path(), "/api/ping", false).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-security-policy").unwrap(), OFFLINE_CSP);
    assert!(body.is_empty());
    assert_eq!(api_status, StatusCode::OK);
    assert_eq!(api_body.as_slice(), b"pong");
}
