//! Платформенный контракт видимости приложения для управления постоянным соединением.

mod native;

use futures_util::stream::LocalBoxStream;

/// Источник событий видимости; первая запись содержит текущее состояние.
trait ApplicationLifecycleBackend {
    /// Отправляет текущее состояние и последующие изменения видимости.
    fn subscribe_visibility() -> LocalBoxStream<'static, bool>;
}

/// Подписывается на видимость Android Activity; остальные платформы остаются активными.
pub(crate) fn subscribe_visibility() -> LocalBoxStream<'static, bool> {
    <native::PlatformLifecycle as ApplicationLifecycleBackend>::subscribe_visibility()
}
