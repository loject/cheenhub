//! Гарантия единственного экземпляра desktop-клиента.

mod native;

pub(crate) use native::SingleInstanceEffects;

/// Регистрирует текущий процесс или активирует уже запущенный экземпляр.
///
/// Возвращает `true`, когда текущий процесс должен запустить приложение.
pub(crate) fn prepare() -> Result<bool, String> {
    native::prepare()
}
