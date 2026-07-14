//! Идентичность auth HTTP-клиента для Linux.

/// Возвращает User-Agent нативного Linux-клиента.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    Some(concat!("CheenHub/", env!("CARGO_PKG_VERSION"), " (Linux)"))
}

/// Возвращает название платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "linux"
}
