//! HTTP-проверки ACME и редиректа на канонический HTTPS URL.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{HeaderMap, Method, StatusCode},
};

use super::router;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("cheenhub-edge-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join(".well-known/acme-challenge")).unwrap();
        Self(root)
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
    app: Router,
    method: Method,
    path: &str,
    host: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let response = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap()
        .request(method, format!("http://{address}{path}"))
        .header("host", host)
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
async fn redirects_to_canonical_https_origin_and_preserves_query() {
    let fixture = Fixture::new();
    let app = router("https://cheenhub.test/api", fixture.path()).unwrap();
    let (status, headers, _) = request(
        app,
        Method::GET,
        "/login?next=%2Fsettings",
        "attacker.invalid",
    )
    .await;

    assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(
        headers.get("location").unwrap(),
        "https://cheenhub.test/login?next=%2Fsettings"
    );
}

#[tokio::test]
async fn acme_token_served_without_redirect_or_cache() {
    let fixture = Fixture::new();
    fs::write(
        fixture
            .path()
            .join(".well-known/acme-challenge/sample_token-123"),
        b"acme-proof",
    )
    .unwrap();
    let app = router("https://cheenhub.test/api", fixture.path()).unwrap();
    let (status, headers, body) = request(
        app.clone(),
        Method::GET,
        "/.well-known/acme-challenge/sample_token-123",
        "cheenhub.test",
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"acme-proof");
    assert_eq!(
        headers.get("content-type").unwrap(),
        "text/plain; charset=utf-8"
    );
    assert_eq!(headers.get("cache-control").unwrap(), "no-store");
    assert!(headers.get("location").is_none());

    let (status, _, body) = request(
        app,
        Method::HEAD,
        "/.well-known/acme-challenge/sample_token-123",
        "cheenhub.test",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.is_empty());
}

#[tokio::test]
async fn missing_acme_token_returns_404() {
    let fixture = Fixture::new();
    let app = router("https://cheenhub.test/api", fixture.path()).unwrap();
    let (status, headers, _) = request(
        app,
        Method::GET,
        "/.well-known/acme-challenge/missing",
        "cheenhub.test",
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(headers.get("location").is_none());
}

#[test]
fn requires_https_origin_and_existing_webroot() {
    let fixture = Fixture::new();
    assert!(router("http://cheenhub.test/api", fixture.path()).is_err());
    assert!(router("https://cheenhub.test/api", &fixture.path().join("missing")).is_err());
}
