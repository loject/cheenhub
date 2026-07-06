//! Платформенная доступность уведомлений о новых версиях приложения.

mod native;

/// Возвращает, нужно ли показывать уведомления о новых версиях приложения.
pub(super) const fn application_update_notifications_enabled() -> bool {
    native::application_update_notifications_enabled()
}
