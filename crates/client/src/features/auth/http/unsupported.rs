//! Идентичность auth HTTP-клиента для неподдерживаемой платформы.

/// Не задает User-Agent без достоверной информации о платформе.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    None
}

/// Возвращает название неподдерживаемой платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "unsupported"
}
