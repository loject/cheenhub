//! Идентичность auth HTTP-клиента для Windows.

/// Возвращает User-Agent нативного Windows-клиента.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    Some(concat!(
        "CheenHub/",
        env!("CARGO_PKG_VERSION"),
        " (Windows)"
    ))
}

/// Возвращает название платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "windows"
}
