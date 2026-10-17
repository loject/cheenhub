//! Проверки лимита по TCP IP, восстановления токенов и защиты от подмены адреса.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

use axum::{
    Router,
    http::StatusCode,
    middleware,
    routing::{get, post},
};

use super::Limiter;

#[test]
fn burst_refills_per_ip_at_five_requests_per_second() {
    let limiter = Limiter::new();
    let now = Instant::now();
    let first = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    let second = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));

    for _ in 0..11 {
        assert!(limiter.allow(first, now));
    }
    assert!(!limiter.allow(first, now));
    assert!(limiter.allow(second, now));
    assert!(limiter.allow(first, now + Duration::from_millis(200)));
    assert!(!limiter.allow(first, now + Duration::from_millis(200)));
}

#[test]
fn full_ip_table_reclaims_idle_entries_after_timeout() {
    let limiter = Limiter::with_max_ips(2);
    let now = Instant::now();
    let a = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    let b = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));
    let c = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 3));

    assert!(limiter.allow(a, now));
    assert!(limiter.allow(b, now));
    assert!(!limiter.allow(c, now));
    assert!(limiter.allow(c, now + Duration::from_secs(61)));
}

#[tokio::test]
async fn forged_forwarded_headers_do_not_bypass_tcp_ip_limit() {
    let app = Router::new()
        .route("/api/auth/login", post(|| async { "ok" }))
        .route("/health", get(|| async { "healthy" }))
        .layer(middleware::from_fn_with_state(
            Limiter::new(),
            super::enforce,
        ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let mut rejected = false;
    for index in 0..30 {
        let response = client
            .post(format!("http://{address}/api/auth/login"))
            .header("x-forwarded-for", format!("203.0.113.{}", index + 1))
            .send()
            .await
            .unwrap();
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            assert_eq!(response.headers().get("retry-after").unwrap(), "1");
            rejected = true;
            break;
        }
        assert_eq!(response.status(), StatusCode::OK);
    }
    let health = client
        .get(format!("http://{address}/health"))
        .send()
        .await
        .unwrap();

    assert!(
        rejected,
        "смена заголовка не должна сбрасывать лимит TCP IP"
    );
    assert_eq!(health.status(), StatusCode::OK);
    server.abort();
}

#[tokio::test]
async fn missing_connect_info_rejects_auth_but_not_health() {
    let app = Router::new()
        .route("/api/auth/login", post(|| async { "ok" }))
        .route("/health", get(|| async { "healthy" }))
        .layer(middleware::from_fn_with_state(
            Limiter::new(),
            super::enforce,
        ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let auth = client
        .post(format!("http://{address}/api/auth/login"))
        .send()
        .await
        .unwrap();
    let health = client
        .get(format!("http://{address}/health"))
        .send()
        .await
        .unwrap();

    assert_eq!(auth.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(health.status(), StatusCode::OK);
    server.abort();
}
