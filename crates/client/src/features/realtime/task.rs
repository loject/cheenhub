//! Realtime task spawning helpers.

/// Spawns a realtime background task.
pub(crate) fn spawn_task(future: impl std::future::Future<Output = ()> + 'static) {
    dioxus::prelude::spawn(future);
}
