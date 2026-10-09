//! Заглушка глобального ввода для неподдерживаемой платформы.

use super::super::key::PushToTalkKey;

/// Недоступный источник глобального ввода; не открывает gate.
pub(in crate::features::microphone) struct Monitor;
impl Monitor {
    /// Отклоняет запуск глобального ввода на неподдерживаемой платформе.
    pub(in crate::features::microphone) fn start(
        _: PushToTalkKey,
        _: std::sync::Arc<std::sync::atomic::AtomicU64>,
    ) -> Result<Self, &'static str> {
        Err(unsupported_reason())
    }
    /// Недоступный ввод не имеет активного поколения удержания.
    pub(in crate::features::microphone) fn held_epoch(&self) -> Option<(u64, u64)> {
        None
    }
}
/// Сообщает, что глобальный ввод недоступен.
pub(in crate::features::microphone) fn supported() -> bool {
    false
}
/// Объясняет ограничение платформы пользователю.
pub(in crate::features::microphone) fn unsupported_reason() -> &'static str {
    "Глобальный Push-to-talk доступен в приложении для Windows."
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
