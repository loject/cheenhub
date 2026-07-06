//! Политика уведомлений о новых версиях для неподдерживаемых платформ.

/// Возвращает, нужно ли показывать уведомления о новых версиях приложения.
pub(super) const fn application_update_notifications_enabled() -> bool {
    false
}
