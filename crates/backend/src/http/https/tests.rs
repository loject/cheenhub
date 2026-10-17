//! Проверка реального HTTPS-запроса через отдельный TCP-слушатель.

use std::{fs, net::SocketAddr};

use axum::{Router, http::StatusCode, routing::get};

use super::start;

#[tokio::test]
async fn serves_https_with_hsts_and_shuts_down() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-https-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&directory).unwrap();
    let certificate = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_path = directory.join("cert.pem");
    let key_path = directory.join("key.pem");
    fs::write(&cert_path, certificate.cert.pem()).unwrap();
    fs::write(&key_path, certificate.key_pair.serialize_pem()).unwrap();
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let listener = start(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        cert_path.to_str().unwrap(),
        key_path.to_str().unwrap(),
        app,
    )
    .await
    .unwrap();
    let address = listener.handle.listening().await.unwrap();

    let response = reqwest::Client::builder()
        .no_proxy()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(format!("https://{address}/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("strict-transport-security").unwrap(),
        "max-age=63072000; includeSubDomains"
    );
    assert_eq!(response.text().await.unwrap(), "ok");
    listener.shutdown().await.unwrap();
    let _ = fs::remove_dir_all(directory);
}

#[tokio::test]
async fn rejects_non_matching_certificate_and_key() {
    let directory =
        std::env::temp_dir().join(format!("cheenhub-https-keys-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&directory).unwrap();
    let first = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let second = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let cert_path = directory.join("cert.pem");
    let key_path = directory.join("key.pem");
    fs::write(&cert_path, first.cert.pem()).unwrap();
    fs::write(&key_path, second.key_pair.serialize_pem()).unwrap();

    let result = start(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        cert_path.to_str().unwrap(),
        key_path.to_str().unwrap(),
        Router::new(),
    )
    .await;

    assert!(result.is_err());
    let _ = fs::remove_dir_all(directory);
}
