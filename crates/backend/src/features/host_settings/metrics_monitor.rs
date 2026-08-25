//! Получение и хранение краткой истории системных метрик хоста.

use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cheenhub_contracts::rest::{HostMetricsResponse, HostMetricsSample};
use tokio::sync::RwLock;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const MAX_SAMPLES: usize = 150;
const STALE_AFTER: Duration = Duration::from_secs(10);

/// Оперативная история нагрузки, полученная от изолированного metrics proxy.
pub(crate) struct HostMetricsMonitor {
    proxy_url: Option<String>,
    client: reqwest::Client,
    samples: RwLock<VecDeque<HostMetricsSample>>,
    connected: AtomicBool,
}

impl HostMetricsMonitor {
    /// Создаёт монитор для настроенного внутреннего proxy URL.
    pub(crate) fn new(proxy_url: Option<String>) -> Self {
        Self {
            proxy_url: proxy_url.map(|url| url.trim_end_matches('/').to_owned()),
            client: reqwest::Client::new(),
            samples: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
            connected: AtomicBool::new(false),
        }
    }

    /// Создаёт отключённый монитор для тестовых состояний приложения.
    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self::new(None)
    }

    /// Периодически обновляет историю до завершения процесса backend.
    pub(crate) async fn run(self: Arc<Self>) {
        let Some(proxy_url) = self.proxy_url.clone() else {
            tracing::warn!(
                missing_env = "CHEENHUB_METRICS_PROXY_URL",
                "host metrics dashboard is unavailable because metrics proxy is not configured"
            );
            return;
        };
        tracing::info!(proxy_url = %proxy_url, "starting host metrics monitor");
        loop {
            self.collect_once(&proxy_url).await;
            tokio::time::sleep(SAMPLE_INTERVAL).await;
        }
    }

    /// Возвращает накопленную историю и признак свежести источника.
    pub(crate) async fn snapshot(&self) -> HostMetricsResponse {
        let samples = self.samples.read().await;
        let fresh = samples.back().is_some_and(|sample| {
            unix_timestamp_millis().saturating_sub(sample.sampled_at_unix_ms)
                <= STALE_AFTER.as_millis() as i64
        });
        HostMetricsResponse {
            available: self.connected.load(Ordering::Relaxed) && fresh,
            samples: samples.iter().cloned().collect(),
        }
    }

    async fn collect_once(&self, proxy_url: &str) {
        let result = self
            .client
            .get(format!("{proxy_url}/v1/metrics"))
            .timeout(SAMPLE_INTERVAL)
            .send()
            .await;
        let response = match result {
            Ok(response) if response.status() == reqwest::StatusCode::NO_CONTENT => return,
            Ok(response) if response.status().is_success() => response,
            Ok(response) => {
                self.record_failure(&format!("HTTP {}", response.status()));
                return;
            }
            Err(error) => {
                self.record_failure(&error.to_string());
                return;
            }
        };
        let sample = match response.json::<HostMetricsSample>().await {
            Ok(sample) => sample,
            Err(error) => {
                self.record_failure(&error.to_string());
                return;
            }
        };
        let mut samples = self.samples.write().await;
        if samples.len() == MAX_SAMPLES {
            samples.pop_front();
        }
        samples.push_back(sample);
        if !self.connected.swap(true, Ordering::Relaxed) {
            tracing::info!("host metrics proxy connection established");
        }
    }

    fn record_failure(&self, error: &str) {
        if self.connected.swap(false, Ordering::Relaxed) {
            tracing::warn!(%error, "host metrics proxy connection lost");
        } else {
            tracing::debug!(%error, "host metrics proxy is not ready");
        }
    }
}

fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
