//! Защита авторизации на прямом HTTPS-подключении.
//!
//! IP определяется по TCP-соединению, а не по клиентским HTTP-заголовкам.
//! Обычный HTTP за Nginx продолжает использовать ограничения самого Nginx.

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State, connect_info::ConnectInfo},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

const RATE_PER_SECOND: f64 = 5.0;
const BURST_CAPACITY: f64 = 11.0;
const MAX_IPS: usize = 65_536;
const IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(5);

struct Bucket {
    tokens: f64,
    seen_at: Instant,
}

struct Ledger {
    buckets: HashMap<IpAddr, Bucket>,
    cleaned_at: Instant,
}

#[derive(Clone)]
pub(super) struct Limiter {
    ledger: Arc<Mutex<Ledger>>,
    max_ips: usize,
}

impl Limiter {
    pub(super) fn new() -> Self {
        tracing::info!(
            rate_per_second = 5,
            burst = 10,
            max_ips = MAX_IPS,
            "включён HTTPS rate limit авторизации"
        );
        Self::with_max_ips(MAX_IPS)
    }

    fn with_max_ips(max_ips: usize) -> Self {
        Self {
            ledger: Arc::new(Mutex::new(Ledger {
                buckets: HashMap::new(),
                cleaned_at: Instant::now(),
            })),
            max_ips,
        }
    }

    fn allow(&self, ip: IpAddr, now: Instant) -> bool {
        let mut ledger = self
            .ledger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(bucket) = ledger.buckets.get_mut(&ip) {
            let seconds = now.saturating_duration_since(bucket.seen_at).as_secs_f64();
            bucket.tokens = (bucket.tokens + seconds * RATE_PER_SECOND).min(BURST_CAPACITY);
            bucket.seen_at = now;
            if bucket.tokens < 1.0 {
                return false;
            }
            bucket.tokens -= 1.0;
            return true;
        }
        if ledger.buckets.len() >= self.max_ips {
            if now.saturating_duration_since(ledger.cleaned_at) >= CLEANUP_INTERVAL {
                ledger.buckets.retain(|_, bucket| {
                    now.saturating_duration_since(bucket.seen_at) < IDLE_TIMEOUT
                });
                ledger.cleaned_at = now;
            }
            if ledger.buckets.len() >= self.max_ips {
                tracing::warn!(
                    max_ips = self.max_ips,
                    "таблица IP HTTPS rate limit заполнена"
                );
                return false;
            }
        }
        ledger.buckets.insert(
            ip,
            Bucket {
                tokens: BURST_CAPACITY - 1.0,
                seen_at: now,
            },
        );
        true
    }
}

/// Ограничивает `/api/auth/*` по достоверному адресу TCP-подключения.
///
/// При отсутствии информации о TCP peer отклоняет авторизацию с HTTP 503.
/// Значения `X-Forwarded-For` и `X-Real-IP` не используются.
pub(super) async fn enforce(
    State(limiter): State<Limiter>,
    request: Request,
    next: Next,
) -> Response {
    if !request.uri().path().starts_with("/api/auth/") {
        return next.run(request).await;
    }
    let Some(ip) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| peer.0.ip())
    else {
        tracing::error!(path = %request.uri().path(), "для HTTPS авторизации отсутствует адрес TCP peer");
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    if limiter.allow(ip, Instant::now()) {
        return next.run(request).await;
    }
    tracing::debug!(%ip, path = %request.uri().path(), "превышена частота HTTPS запросов авторизации");
    let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}

#[cfg(test)]
mod tests;
