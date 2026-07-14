//! Выбор идентичности auth HTTP-клиента для текущей платформы.

#[cfg(feature = "web")]
#[path = "web.rs"]
mod platform;

#[cfg(feature = "windows")]
#[path = "windows.rs"]
mod platform;

#[cfg(feature = "linux")]
#[path = "linux.rs"]
mod platform;

#[cfg(feature = "macos")]
#[path = "macos.rs"]
mod platform;

#[cfg(target_os = "android")]
#[path = "android.rs"]
mod platform;

#[cfg(not(any(
    feature = "web",
    feature = "windows",
    feature = "linux",
    feature = "macos",
    target_os = "android"
)))]
#[path = "unsupported.rs"]
mod platform;

/// Возвращает User-Agent нативного клиента или сохраняет браузерное значение.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    platform::client_user_agent()
}

/// Возвращает название текущей платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    platform::client_platform()
}
