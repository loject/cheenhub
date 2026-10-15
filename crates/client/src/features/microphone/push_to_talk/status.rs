//! Состояние системного PTT для настроек; готовность capture остаётся в platform gate.

use super::PushToTalkKey;

/// Передаёт изменения platform worker в microphone provider на UI-потоке.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Update {
    /// Результат проверки API: None — проверка идёт, Some(true) — доступен.
    pub(crate) availability: Option<bool>,
    /// Нужно назначить системное сочетание; передача остановлена.
    pub(crate) binding_required: bool,
    /// Последняя ошибка системного управления, если она есть.
    pub(crate) error: Option<String>,
    /// Последнее подтверждённое назначение; None не меняет сохранённую настройку.
    pub(crate) binding: Option<PushToTalkKey>,
    /// Открыто переназначение без события завершения; UI предлагает «Готово» и отмену.
    pub(crate) configuring: bool,
    /// Авария worker оставила capture со старым Monitor; нужно явно перезапустить микрофон.
    pub(crate) restart_capture: bool,
}
