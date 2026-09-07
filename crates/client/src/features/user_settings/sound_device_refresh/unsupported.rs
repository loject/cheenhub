//! Платформы с загрузкой устройств только при открытии настроек.

/// Таймер без периодического обновления.
pub(crate) struct Timer;

impl super::super::RefreshTimer for Timer {
    async fn wait() {
        std::future::pending::<()>().await;
    }
}
