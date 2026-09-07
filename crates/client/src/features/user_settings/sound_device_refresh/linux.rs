//! Периодическая проверка устройств Linux в открытых настройках.

/// Таймер обновления устройств PulseAudio.
pub(crate) struct Timer;

impl super::super::RefreshTimer for Timer {
    async fn wait() {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
