//! Проверки точных маршрутов публичных документов.

use std::{
    fs,
    path::{Path, PathBuf},
};

use axum::{
    Router,
    http::{HeaderMap, StatusCode},
    routing::get,
};

use super::mount_documents;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cheenhub-public-documents-test-{}",
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

async fn request(directory: &Path, route: &str, head: bool) -> (StatusCode, HeaderMap, Vec<u8>) {
    let app = mount_documents(
        Router::<()>::new()
            .route("/api/ping", get(|| async { "pong" }))
            .fallback(|| async { StatusCode::IM_A_TEAPOT }),
        directory,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let url = format!("http://{address}{route}");
    let response = if head {
        client.head(url).send().await.unwrap()
    } else {
        client.get(url).send().await.unwrap()
    };
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.unwrap().to_vec();
    task.abort();
    (status, headers, body)
}

#[tokio::test]
async fn public_documents_have_correct_mime_type_and_cache_headers() {
    let fixture = Fixture::new();
    for (name, mime) in [
        ("manifest.webmanifest", "application/manifest+json"),
        ("robots.txt", "text/plain; charset=utf-8"),
        ("sitemap.xml", "application/xml"),
    ] {
        fs::write(fixture.path().join(name), format!("document:{name}")).unwrap();
        let (status, headers, body) = request(fixture.path(), &format!("/{name}"), false).await;
        assert_eq!(status, StatusCode::OK, "{name}");
        assert_eq!(
            headers.get("content-type").unwrap().to_str().unwrap(),
            mime,
            "{name}"
        );
        assert_eq!(headers.get("cache-control").unwrap(), "no-cache", "{name}");
        assert_eq!(
            headers.get("x-content-type-options").unwrap(),
            "nosniff",
            "{name}"
        );
        assert_eq!(
            body.as_slice(),
            format!("document:{name}").as_bytes(),
            "{name}"
        );
    }
}

#[tokio::test]
async fn missing_document_keeps_existing_fallback() {
    let fixture = Fixture::new();

    let (status, _, _) = request(fixture.path(), "/manifest.webmanifest", false).await;

    assert_eq!(status, StatusCode::IM_A_TEAPOT);
}

#[tokio::test]
async fn documents_do_not_intercept_api() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("robots.txt"), "User-agent: *").unwrap();

    let (status, _, body) = request(fixture.path(), "/api/ping", false).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_slice(), b"pong");
}

#[tokio::test]
async fn head_returns_headers_without_body() {
    let fixture = Fixture::new();
    fs::write(fixture.path().join("manifest.webmanifest"), "{}").unwrap();

    let (status, headers, body) = request(fixture.path(), "/manifest.webmanifest", true).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        headers.get("content-type").unwrap(),
        "application/manifest+json"
    );
    assert!(body.is_empty());
}
