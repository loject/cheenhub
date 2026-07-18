//! Управление автоматическим запуском клиентского приложения.

mod handle;
mod native;
mod provider;

pub(crate) use handle::AutostartHandle;
pub(crate) use provider::AutostartProvider;

/// Возвращает, был ли запрошен скрытый запуск приложения вместе с системой.
pub(crate) fn started_hidden() -> bool {
    native::started_hidden()
}
