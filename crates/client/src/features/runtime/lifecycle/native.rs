//! Выбор источника событий жизненного цикла приложения.

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod android;

/// Источник жизненного цикла для текущей платформы.
pub(super) struct PlatformLifecycle;

impl super::ApplicationLifecycleBackend for PlatformLifecycle {
    #[cfg(target_os = "android")]
    fn subscribe_visibility() -> futures_util::stream::LocalBoxStream<'static, bool> {
        android::subscribe_visibility()
    }

    #[cfg(not(target_os = "android"))]
    fn subscribe_visibility() -> futures_util::stream::LocalBoxStream<'static, bool> {
        use futures_util::{StreamExt, stream};
        stream::once(async { true })
            .chain(stream::pending())
            .boxed_local()
    }
}
