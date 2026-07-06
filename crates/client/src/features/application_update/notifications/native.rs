//! Выбор платформенной политики уведомлений о новых версиях приложения.

#[cfg(target_arch = "wasm32")]
#[path = "web.rs"]
mod platform;

#[cfg(all(feature = "desktop", not(target_arch = "wasm32")))]
#[path = "desktop.rs"]
mod platform;

#[cfg(all(not(feature = "desktop"), not(target_arch = "wasm32")))]
#[path = "unsupported.rs"]
mod platform;

/// Возвращает, нужно ли показывать уведомления о новых версиях приложения.
pub(super) const fn application_update_notifications_enabled() -> bool {
    platform::application_update_notifications_enabled()
}
