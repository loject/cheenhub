//! Идентичность auth HTTP-клиента для Android.

/// Возвращает User-Agent нативного Android-клиента.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    Some(concat!(
        "CheenHub/",
        env!("CARGO_PKG_VERSION"),
        " (Android)"
    ))
}

/// Возвращает название платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "android"
}
