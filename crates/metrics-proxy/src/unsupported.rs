//! Сохраняет API Linux-сборщиков для сборки backend на других ОС.

use cheenhub_contracts::rest::HostMetricsSample;

/// Недоступный на этой ОС прямой сборщик метрик Linux-хоста.
///
/// Создание разрешено, но каждый вызов `collect` возвращает ошибку поддержки ОС.
pub struct NativeMetricsCollector;

impl NativeMetricsCollector {
    /// Создаёт сборщик без обращения к файловой системе; путь на этой ОС не используется.
    pub fn new(_disk_path: String) -> Self {
        tracing::warn!(
            source = "native",
            os = std::env::consts::OS,
            "host metrics require Linux"
        );
        Self
    }

    /// Сообщает о недоступности прямого сбора метрик.
    ///
    /// # Errors
    /// Всегда возвращает ошибку: прямой сбор поддерживается только на Linux.
    pub async fn collect(&mut self) -> anyhow::Result<Option<HostMetricsSample>> {
        anyhow::bail!("native host metrics are supported only on Linux")
    }
}

/// Недоступный на этой ОС сборщик метрик Linux-хоста через Docker socket.
///
/// Сохраняет контракт proxy; подключение к Docker на неподдерживаемой ОС не выполняется.
pub struct DockerMetricsCollector;

impl DockerMetricsCollector {
    /// Создаёт сборщик без подключения к Docker; параметры на этой ОС не используются.
    pub fn new(
        _socket_path: String,
        _app_services: Vec<String>,
        _database_service: String,
        _disk_path: String,
    ) -> Self {
        tracing::warn!(
            source = "docker",
            os = std::env::consts::OS,
            "host metrics require Linux"
        );
        Self
    }

    /// Сообщает о недоступности сбора метрик через Docker socket.
    ///
    /// # Errors
    /// Всегда возвращает ошибку: Docker-сборщик поддерживается только на Linux.
    pub async fn collect(&mut self) -> anyhow::Result<Option<HostMetricsSample>> {
        anyhow::bail!("Docker host metrics are supported only on Linux")
    }
}
