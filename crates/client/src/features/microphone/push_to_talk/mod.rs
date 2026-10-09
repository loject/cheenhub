//! Глобальная активация микрофона по удержанию клавиши; управление звонком остаётся у voice_chat.

/// Правила открытия gate по событиям удержания и восстановления ввода.
pub(super) mod gate;
mod key;
/// Платформенный контракт глобального ввода, записи кнопки и доступности функции.
pub(super) mod platform;

pub(crate) use key::PushToTalkKey;

/// Сообщает, доступен ли глобальный ввод на текущей платформе.
pub(crate) fn supported() -> bool {
    platform::supported()
}

/// Объясняет недоступность глобального ввода в интерфейсе настроек.
pub(crate) fn unsupported_reason() -> &'static str {
    platform::unsupported_reason()
}

/// Записывает одиночную кнопку после её отпускания на поддерживаемой платформе.
///
/// # Errors
/// Возвращает объяснение недоступности ввода, ошибки hooks или времени ожидания.
pub(crate) async fn record_binding() -> Result<PushToTalkKey, String> {
    platform::record_binding().await
}
