//! Вспомогательные функции запуска realtime-задач.

/// Запускает фоновую realtime-задачу.
pub(crate) fn spawn_task(future: impl std::future::Future<Output = ()> + 'static) {
    dioxus::prelude::spawn(future);
}
