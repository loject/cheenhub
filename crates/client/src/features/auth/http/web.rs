//! Идентичность auth HTTP-клиента для браузера.

/// Сохраняет браузерный User-Agent, которым fetch управляет самостоятельно.
pub(super) const fn client_user_agent() -> Option<&'static str> {
    None
}

/// Возвращает название браузерной платформы для диагностики.
pub(super) const fn client_platform() -> &'static str {
    "web"
}
