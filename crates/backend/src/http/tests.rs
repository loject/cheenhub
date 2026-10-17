//! Сквозные проверки совместной работы статических маршрутов HTTP.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{HeaderMap, Method, StatusCode, header},
    routing::get,
};

use super::mount_static_site;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cheenhub-http-site-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn put(&self, name: &str, contents: &[u8]) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn request(directory: &Path, method: Method, path: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    let app = mount_static_site(
        Router::<()>::new()
            .route("/health", get(|| async { "healthy" }))
            .nest(
                "/api",
                Router::new()
                    .route("/ping", get(|| async { "pong" }))
                    .fallback(|| async { StatusCode::NOT_FOUND }),
            )
            .fallback(|| async { StatusCode::IM_A_TEAPOT }),
        directory,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .request(method, format!("http://{address}{path}"))
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
async fn html_routes_keep_their_distinct_security_policies() {
    let fixture = Fixture::new();
    fixture.put("index.html", b"app");
    fixture.put("landing.html", b"landing");
    fixture.put("offline.html", b"offline");

    for (path, body, script_rule) in [
        ("/", "landing", "script-src 'none'"),
        ("/index.html", "app", "script-src 'self' 'wasm-unsafe-eval'"),
        (
            "/app/settings",
            "app",
            "script-src 'self' 'wasm-unsafe-eval'",
        ),
        ("/offline.html", "offline", "script-src 'self'"),
    ] {
        let (status, headers, actual) = request(fixture.path(), Method::GET, path).await;

        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(actual.as_slice(), body.as_bytes(), "{path}");
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
        assert!(
            headers
                .get("content-security-policy")
                .unwrap()
                .to_str()
                .unwrap()
                .contains(script_rule),
            "{path}"
        );
        assert_eq!(headers.get("x-frame-options").unwrap(), "DENY");
        assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");
    }
}

#[tokio::test]
async fn pwa_routes_keep_their_content_types_and_cache_rules() {
    let fixture = Fixture::new();
    for filename in [
        "sw.js",
        "pwa-register.js",
        "manifest.webmanifest",
        "robots.txt",
        "sitemap.xml",
    ] {
        fixture.put(filename, b"content");
    }

    for (path, content_type, cache) in [
        (
            "/sw.js",
            "application/javascript; charset=utf-8",
            "no-cache, no-store, must-revalidate",
        ),
        (
            "/pwa-register.js",
            "application/javascript; charset=utf-8",
            "no-cache",
        ),
        (
            "/manifest.webmanifest",
            "application/manifest+json",
            "no-cache",
        ),
        ("/robots.txt", "text/plain; charset=utf-8", "no-cache"),
        ("/sitemap.xml", "application/xml", "no-cache"),
    ] {
        let (status, headers, body) = request(fixture.path(), Method::GET, path).await;

        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body.as_slice(), b"content", "{path}");
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            content_type,
            "{path}"
        );
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), cache, "{path}");
        if path == "/sw.js" {
            assert_eq!(headers.get("service-worker-allowed").unwrap(), "/");
        }
    }
}

#[tokio::test]
async fn static_routes_keep_separate_cache_policies_and_missing_files_are_not_cached() {
    let fixture = Fixture::new();
    for filename in [
        "assets/app.js",
        "wasm/app.wasm",
        "icons/logo.svg",
        "screenshots/demo.png",
    ] {
        fixture.put(filename, b"content");
    }

    for (path, cache) in [
        ("/assets/app.js", "public, max-age=31536000, immutable"),
        ("/wasm/app.wasm", "public, max-age=31536000, immutable"),
        ("/icons/logo.svg", "public, max-age=86400"),
        ("/screenshots/demo.png", "public, max-age=86400"),
    ] {
        let (status, headers, body) = request(fixture.path(), Method::GET, path).await;

        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body.as_slice(), b"content", "{path}");
        assert_eq!(headers.get(header::CACHE_CONTROL).unwrap(), cache, "{path}");
    }

    for path in [
        "/assets/missing.js",
        "/wasm/missing.wasm",
        "/icons/missing.svg",
        "/screenshots/missing.png",
    ] {
        let (status, headers, _) = request(fixture.path(), Method::GET, path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
}

#[tokio::test]
async fn api_and_health_are_not_shadowed_by_spa_fallback() {
    let fixture = Fixture::new();
    fixture.put("index.html", b"app");

    for (method, path, expected) in [
        (Method::GET, "/api/ping", StatusCode::OK),
        (Method::GET, "/api/missing", StatusCode::NOT_FOUND),
        (Method::GET, "/health", StatusCode::OK),
        (Method::GET, "/missing.js", StatusCode::NOT_FOUND),
        (Method::POST, "/app/settings", StatusCode::NOT_FOUND),
    ] {
        let (status, headers, _) = request(fixture.path(), method, path).await;
        assert_eq!(status, expected, "{path}");
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
}

#[tokio::test]
async fn missing_site_preserves_original_fallback_and_api() {
    let fixture = Fixture::new();
    let missing = fixture.path().join("no-web-public");

    for path in ["/", "/index.html", "/app", "/sw.js"] {
        let (status, _, _) = request(&missing, Method::GET, path).await;
        assert_eq!(status, StatusCode::IM_A_TEAPOT, "{path}");
    }
    let (status, _, body) = request(&missing, Method::GET, "/api/ping").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_slice(), b"pong");
}
