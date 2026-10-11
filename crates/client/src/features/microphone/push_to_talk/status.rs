//! Состояние системного PTT для настроек; готовность capture остаётся в platform gate.

use super::PushToTalkKey;

/// Результат первоначальной технической проверки Portal, отдельно от PTT-сессии.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Availability {
    /// Проверка ещё не завершена; gate остаётся закрытым.
    Pending,
    /// CreateSession подтвердил поддержку backend.
    Available,
    /// Проверка завершилась ошибкой; возможно повторное подключение.
    Unavailable,
}

/// Разделяет доступность системного API, назначение и готовность PTT-сессии.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    /// Portal доступен, но активной PTT-сессии нет.
    Available,
    /// Требуется явное назначение пользователем; gate закрыт.
    BindingRequired,
    /// Сессия зарегистрирована; передача требует нового Activated.
    Ready,
    /// Ошибка закрыла gate; сообщение предназначено пользователю.
    Failed(String),
}

/// Передаёт изменения platform worker в microphone provider на UI-потоке.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Update {
    /// Фактическая поддержка API; не означает готовность передачи.
    pub(crate) availability: Availability,
    /// Текущее состояние системного PTT.
    pub(crate) status: Status,
    /// Последнее подтверждённое назначение; None не меняет сохранённую настройку.
    pub(crate) binding: Option<PushToTalkKey>,
    /// Открыто переназначение без события завершения; UI предлагает «Готово» и отмену.
    pub(crate) configuring: bool,
    /// Авария worker оставила capture со старым Monitor; нужно явно перезапустить микрофон.
    pub(crate) restart_capture: bool,
}
