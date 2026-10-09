//! Заглушка глобального ввода для неподдерживаемой платформы.

/// Сообщает, что глобальный ввод недоступен.
pub(in crate::features::microphone) fn supported() -> bool {
    false
}
/// Объясняет ограничение платформы пользователю.
pub(in crate::features::microphone) fn unsupported_reason() -> &'static str {
    "Глобальный Push-to-talk недоступен в браузере. Используйте приложение для Windows."
}

/// Даёт платформенно нейтральное название сохранённой кнопки.
pub(in crate::features::microphone) fn key_label(key: super::super::key::PushToTalkKey) -> String {
    key.fallback_label()
}

/// Отклоняет назначение глобального ввода на неподдерживаемой платформе.
///
/// # Errors
/// Всегда возвращает объяснение платформенной недоступности.
pub(in crate::features::microphone) async fn record_binding()
-> Result<super::super::key::PushToTalkKey, String> {
    Err(unsupported_reason().into())
}
