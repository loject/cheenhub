//! Идентичность auth HTTP-клиента для macOS.

/// Возвращает User-Agent нативного macOS-клиента.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    Some(concat!("CheenHub/", env!("CARGO_PKG_VERSION"), " (macOS)"))
}

/// Возвращает название платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "macos"
}
