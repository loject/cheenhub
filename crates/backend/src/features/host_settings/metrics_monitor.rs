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
use cheenhub_metrics_proxy::{NativeMetricsCollector, running_in_container};
use tokio::sync::RwLock;

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const MAX_SAMPLES: usize = 150;
const STALE_AFTER: Duration = Duration::from_secs(10);

/// Оперативная история нагрузки Linux-хоста или изолированного Docker metrics proxy.
pub(crate) struct HostMetricsMonitor {
    source: MetricsSource,
    client: reqwest::Client,
    samples: RwLock<VecDeque<HostMetricsSample>>,
    connected: AtomicBool,
    failure_reported: AtomicBool,
}

#[derive(Debug, PartialEq)]
enum MetricsSource {
    Native,
    Proxy(String),
    Unavailable,
}

fn select_source(container: bool, proxy_url: Option<String>) -> MetricsSource {
    if container {
        proxy_url.map_or(MetricsSource::Unavailable, MetricsSource::Proxy)
    } else {
        MetricsSource::Native
    }
}

impl HostMetricsMonitor {
    /// Выбирает прямой сбор вне контейнера и изолированный proxy внутри него.
    pub(crate) fn new(proxy_url: Option<String>) -> Self {
        Self {
            source: select_source(
                running_in_container(),
                proxy_url.map(|url| url.trim_end_matches('/').to_owned()),
            ),
            client: reqwest::Client::new(),
            samples: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
            connected: AtomicBool::new(false),
            failure_reported: AtomicBool::new(false),
        }
    }

    /// Создаёт отключённый монитор для тестовых состояний приложения.
    #[cfg(test)]
    pub(crate) fn disabled() -> Self {
        Self {
            source: MetricsSource::Unavailable,
            client: reqwest::Client::new(),
            samples: RwLock::new(VecDeque::with_capacity(MAX_SAMPLES)),
            connected: AtomicBool::new(false),
            failure_reported: AtomicBool::new(false),
        }
    }

    /// Периодически обновляет историю до завершения процесса backend.
    pub(crate) async fn run(self: Arc<Self>) {
        match &self.source {
            MetricsSource::Unavailable => {
                tracing::warn!(
                    missing_env = "CHEENHUB_METRICS_PROXY_URL",
                    source = "docker_proxy",
                    "host metrics unavailable: container deployment requires an isolated metrics proxy"
                );
            }
            MetricsSource::Proxy(proxy_url) => {
                tracing::info!(source = "docker_proxy", "starting host metrics monitor");
                loop {
                    self.collect_once(proxy_url).await;
                    tokio::time::sleep(SAMPLE_INTERVAL).await;
                }
            }
            MetricsSource::Native => {
                let disk_path = std::env::var("CHEENHUB_METRICS_DISK_PATH")
                    .ok()
                    .filter(|path| !path.trim().is_empty())
                    .unwrap_or_else(|| "/".to_owned());
                let mut collector = NativeMetricsCollector::new(disk_path.clone());
                tracing::info!(source = "native", %disk_path, "starting host metrics monitor");
                loop {
                    match collector.collect().await {
                        Ok(Some(sample)) => self.record_sample(sample).await,
                        Ok(None) => {}
                        Err(error) => self.record_failure(&error.to_string()),
                    }
                    tokio::time::sleep(SAMPLE_INTERVAL).await;
                }
            }
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
        self.record_sample(sample).await;
    }

    async fn record_sample(&self, sample: HostMetricsSample) {
        let mut samples = self.samples.write().await;
        if samples.len() == MAX_SAMPLES {
            samples.pop_front();
        }
        samples.push_back(sample);
        self.failure_reported.store(false, Ordering::Relaxed);
        if !self.connected.swap(true, Ordering::Relaxed) {
            tracing::info!("host metrics source is available");
        }
    }

    fn record_failure(&self, error: &str) {
        self.connected.store(false, Ordering::Relaxed);
        if !self.failure_reported.swap(true, Ordering::Relaxed) {
            tracing::warn!(%error, "host metrics source is unavailable");
        } else {
            tracing::debug!(%error, "host metrics source is not ready");
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

#[cfg(test)]
mod tests;
