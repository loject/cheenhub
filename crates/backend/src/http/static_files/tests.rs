//! Проверки изолированных HTTP-маршрутов статических файлов.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{StatusCode, header},
    routing::get,
};

use super::{is_spa_route, mount_asset_dirs, mount_index};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("cheenhub-index-html-test-{}", uuid::Uuid::new_v4()));
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

async fn request(directory: &Path, path: &str) -> (StatusCode, Option<String>, String) {
    let (status, headers, body) = request_with_headers(directory, path).await;
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_owned());
    (status, content_type, body)
}

async fn request_with_headers(
    directory: &Path,
    path: &str,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let app = mount_asset_dirs(
        mount_index(
            Router::<()>::new()
                .route("/api/ping", get(|| async { "pong" }))
                .fallback(|| async { StatusCode::IM_A_TEAPOT }),
            directory,
        ),
        directory,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let response = client
        .get(format!("http://{address}{path}"))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap();
    server.abort();
    (status, headers, body)
}

#[tokio::test]
async fn existing_index_returns_html_and_content_type() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "<h1>CheenHub</h1>").unwrap();

    let (status, content_type, body) = request(fixture.path(), "/index.html").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
    assert_eq!(body, "<h1>CheenHub</h1>");
}

#[tokio::test]
async fn missing_index_in_existing_directory_returns_not_found() {
    let fixture = Fixture::new();

    let (status, _, _) = request(fixture.path(), "/index.html").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn absent_static_directory_keeps_previous_fallback() {
    let fixture = Fixture::new();

    let (status, _, _) = request(&fixture.path().join("absent"), "/index.html").await;

    assert_eq!(status, StatusCode::IM_A_TEAPOT);
}

#[tokio::test]
async fn index_route_does_not_intercept_api() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "html").unwrap();

    let (status, _, body) = request(fixture.path(), "/api/ping").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "pong");
}

#[tokio::test]
async fn nested_javascript_asset_is_served_with_mime_type() {
    let fixture = Fixture::new();
    let assets = fixture.path().join("assets/nested");
    fs::create_dir_all(&assets).unwrap();
    fs::write(assets.join("app.js"), "console.log('ok')").unwrap();

    let (status, content_type, body) = request(fixture.path(), "/assets/nested/app.js").await;

    assert_eq!(status, StatusCode::OK);
    assert!(content_type.is_some_and(|value| value.contains("javascript")));
    assert_eq!(body, "console.log('ok')");
}

#[tokio::test]
async fn stylesheet_is_served_with_css_mime_type() {
    let fixture = Fixture::new();
    let assets = fixture.path().join("assets");
    fs::create_dir(&assets).unwrap();
    fs::write(assets.join("app.css"), "body { color: red; }").unwrap();

    let (status, content_type, body) = request(fixture.path(), "/assets/app.css").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/css"));
    assert_eq!(body, "body { color: red; }");
}

#[tokio::test]
async fn wasm_asset_is_served_with_wasm_mime_type() {
    let fixture = Fixture::new();
    let wasm = fixture.path().join("wasm");
    fs::create_dir(&wasm).unwrap();
    fs::write(wasm.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();

    let (status, content_type, body) = request(fixture.path(), "/wasm/app.wasm").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/wasm"));
    assert_eq!(body.as_bytes(), b"\0asm\x01\0\0\0");
}

#[tokio::test]
async fn unknown_asset_does_not_fall_back_to_html_or_api() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("assets")).unwrap();
    fs::write(fixture.path().join("index.html"), "html").unwrap();

    let (status, _, _) = request(fixture.path(), "/assets/missing.js").await;
    let (api_status, _, api_body) = request(fixture.path(), "/api/ping").await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(api_status, StatusCode::OK);
    assert_eq!(api_body, "pong");
}

#[tokio::test]
async fn assets_and_wasm_have_immutable_cache_on_success() {
    let fixture = Fixture::new();
    let assets = fixture.path().join("assets");
    let wasm = fixture.path().join("wasm");
    fs::create_dir(&assets).unwrap();
    fs::create_dir(&wasm).unwrap();
    fs::write(assets.join("bundle.js"), "const value = 1;").unwrap();
    fs::write(wasm.join("module.wasm"), b"\0asm\x01\0\0\0").unwrap();

    for path in ["/assets/bundle.js", "/wasm/module.wasm"] {
        let (status, headers, _) = request_with_headers(fixture.path(), path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(
            headers.get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=31536000, immutable",
            "{path}"
        );
    }
}

#[tokio::test]
async fn missing_assets_do_not_receive_immutable_cache() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("assets")).unwrap();
    fs::create_dir(fixture.path().join("wasm")).unwrap();

    for path in ["/assets/missing.js", "/wasm/missing.wasm"] {
        let (status, headers, _) = request_with_headers(fixture.path(), path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
}

#[tokio::test]
async fn immutable_cache_is_not_applied_to_api_or_spa() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "<h1>App</h1>").unwrap();
    fs::create_dir(fixture.path().join("assets")).unwrap();

    let (spa_status, spa_headers, _) = request_with_headers(fixture.path(), "/app").await;
    let (api_status, api_headers, _) = request_with_headers(fixture.path(), "/api/ping").await;

    assert_eq!(spa_status, StatusCode::OK);
    assert_eq!(spa_headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
    assert_eq!(api_status, StatusCode::OK);
    assert!(api_headers.get(header::CACHE_CONTROL).is_none());
}

#[tokio::test]
async fn missing_index_keeps_original_spa_fallback() {
    let fixture = Fixture::new();

    let (status, _, _) = request(fixture.path(), "/login").await;

    assert_eq!(status, StatusCode::IM_A_TEAPOT);
}

#[tokio::test]
async fn spa_navigation_returns_index_html() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "<h1>CheenHub</h1>").unwrap();

    let (status, content_type, body) = request(fixture.path(), "/settings/profile").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("text/html; charset=utf-8"));
    assert_eq!(body, "<h1>CheenHub</h1>");
}

#[tokio::test]
async fn root_and_unknown_files_do_not_use_spa_fallback() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "html").unwrap();

    let (root_status, _, _) = request(fixture.path(), "/").await;
    let (file_status, _, _) = request(fixture.path(), "/missing.js").await;
    let (api_status, _, _) = request(fixture.path(), "/api/missing").await;

    assert_eq!(root_status, StatusCode::NOT_FOUND);
    assert_eq!(file_status, StatusCode::NOT_FOUND);
    assert_eq!(api_status, StatusCode::NOT_FOUND);
}

#[test]
fn spa_routes_exclude_unsafe_methods_and_reserved_paths() {
    assert!(is_spa_route(&axum::http::Method::HEAD, "/login"));
    assert!(!is_spa_route(&axum::http::Method::POST, "/login"));
    for path in [
        "/health",
        "/assets/missing",
        "/wasm/missing",
        "/icons/missing",
        "/screenshots/missing",
        "/audio/missing",
        "/workers/missing",
        "/vendor/missing",
        "/api/private",
        "/missing.js",
        "/",
    ] {
        assert!(!is_spa_route(&axum::http::Method::GET, path), "{path}");
    }
}

#[tokio::test]
async fn index_and_spa_responses_have_security_headers() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "<h1>CheenHub</h1>").unwrap();

    for path in ["/index.html", "/settings/profile"] {
        let (status, headers, body) = request_with_headers(fixture.path(), path).await;

        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, "<h1>CheenHub</h1>");
        for (name, expected) in [
            ("content-type", "text/html; charset=utf-8"),
            ("cache-control", "no-cache"),
            ("content-security-policy", super::APP_CSP),
            ("x-content-type-options", "nosniff"),
            ("x-frame-options", "DENY"),
            ("referrer-policy", "strict-origin-when-cross-origin"),
        ] {
            assert_eq!(
                headers.get(name).unwrap().to_str().unwrap(),
                expected,
                "{path}: {name}"
            );
        }
    }

    let (api_status, api_headers, _) = request_with_headers(fixture.path(), "/api/ping").await;
    assert_eq!(api_status, StatusCode::OK);
    assert!(api_headers.get("content-security-policy").is_none());
}

#[tokio::test]
async fn icons_and_screenshots_have_daily_cache_and_correct_mime() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("icons")).unwrap();
    fs::create_dir(fixture.path().join("screenshots")).unwrap();
    fs::write(fixture.path().join("icons/logo.svg"), "<svg/>").unwrap();
    fs::write(fixture.path().join("screenshots/demo.png"), "png").unwrap();

    for (path, expected_type) in [
        ("/icons/logo.svg", "image/svg+xml"),
        ("/screenshots/demo.png", "image/png"),
    ] {
        let (status, headers, _) = request_with_headers(fixture.path(), path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(
            headers.get(header::CONTENT_TYPE).unwrap(),
            expected_type,
            "{path}"
        );
        assert_eq!(
            headers.get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=86400",
            "{path}"
        );
    }
}

#[tokio::test]
async fn missing_media_files_have_no_daily_cache() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.path().join("icons")).unwrap();
    fs::create_dir(fixture.path().join("screenshots")).unwrap();

    for path in ["/icons/missing.svg", "/screenshots/missing.png"] {
        let (status, headers, _) = request_with_headers(fixture.path(), path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
}

#[tokio::test]
async fn audio_workers_and_vendor_files_are_served_with_types_and_revalidation() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "app").unwrap();
    let cases = [
        ("audio/microphone-worklet.js", "?v=4", "text/javascript"),
        (
            "audio/notifications/current-user-joined.wav",
            "",
            "audio/wav",
        ),
        ("workers/microphone/processor.wasm", "", "application/wasm"),
        ("vendor/echarts.min.js", "", "text/javascript"),
    ];
    for (file, query, expected_type) in cases {
        let path = fixture.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "resource").unwrap();

        let (status, headers, body) =
            request_with_headers(fixture.path(), &format!("/{file}{query}")).await;

        assert_eq!(status, StatusCode::OK, "{file}");
        assert_eq!(body, "resource", "{file}");
        assert_eq!(headers[header::CONTENT_TYPE], expected_type, "{file}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-cache", "{file}");
    }
    for path in ["/audio/missing", "/workers/missing", "/vendor/missing"] {
        let (status, headers, _) = request_with_headers(fixture.path(), path).await;

        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(headers.get(header::CACHE_CONTROL).is_none(), "{path}");
    }
}

#[tokio::test]
async fn daily_cache_does_not_affect_spa_or_api() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("index.html"), "app").unwrap();
    fs::create_dir(fixture.path().join("icons")).unwrap();

    let (spa_status, spa_headers, _) = request_with_headers(fixture.path(), "/app").await;
    let (api_status, api_headers, _) = request_with_headers(fixture.path(), "/api/ping").await;
    assert_eq!(spa_status, StatusCode::OK);
    assert_eq!(spa_headers.get(header::CACHE_CONTROL).unwrap(), "no-cache");
    assert_eq!(api_status, StatusCode::OK);
    assert!(api_headers.get(header::CACHE_CONTROL).is_none());
}
