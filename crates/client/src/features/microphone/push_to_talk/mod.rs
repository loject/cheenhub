//! Глобальная активация микрофона по удержанию клавиши; управление звонком остаётся у voice_chat.

/// Правила открытия gate по событиям удержания и восстановления ввода.
pub(super) mod gate;
mod key;
mod status;
pub(crate) use status::{Availability, Status, Update};
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

/// Назначает поддерживаемую платформой кнопку или системное сочетание.
///
/// # Errors
/// Возвращает объяснение недоступности ввода, ошибки hooks или времени ожидания.
pub(crate) async fn record_binding() -> Result<PushToTalkKey, String> {
    platform::record_binding().await
}

/// Проверяет фактическую доступность платформенного источника событий.
pub(crate) async fn availability() -> bool {
    platform::availability().await
}

/// Описывает действия в интерфейсе назначения.
pub(crate) fn binding_prompt() -> &'static str {
    platform::binding_prompt()
}

/// Описывает доступные платформе способы удержания.
pub(crate) fn mode_description() -> &'static str {
    platform::mode_description()
}

/// Описывает ожидание назначения в системном интерфейсе.
pub(crate) fn waiting_label() -> &'static str {
    platform::waiting_label()
}

/// Объясняет использование назначенного сочетания в интерфейсе настроек.
pub(crate) fn hold_description() -> &'static str {
    platform::hold_description()
}

/// Передаёт изменения системной привязки и ошибки в реактивное состояние provider.
pub(crate) async fn observe_updates(on_update: impl FnMut(Update)) {
    platform::observe_updates(on_update).await;
}

/// Завершает системное переназначение с сохранением последнего подтверждённого сочетания.
pub(crate) fn finish_assignment() {
    platform::finish_assignment();
}

/// Повторяет проверку системного API после временного отказа.
pub(crate) fn retry_availability() {
    platform::retry_availability();
}
