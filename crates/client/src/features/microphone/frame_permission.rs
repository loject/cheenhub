//! Отзываемое разрешение, которое сохраняет принадлежность аудио при переходе через локальные очереди.

use std::fmt;
use std::sync::Arc;

/// Разрешение на передачу конкретного кадра; clone сохраняет связь с исходным capture.
///
/// Сравнение проверяет идентичность разрешения, а не текущий результат проверки.
#[derive(Clone)]
pub(crate) struct FramePermission(
    /// Потокобезопасная проверка исходного capture; вызывается без переноса состояния в consumer.
    pub(super) Arc<dyn Fn() -> bool + Send + Sync>,
);

impl FramePermission {
    /// Проверяет, не отозвано ли разрешение перед передачей в следующий consumer.
    pub(crate) fn allowed(&self) -> bool {
        (self.0)()
    }
}

impl fmt::Debug for FramePermission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FramePermission")
    }
}
impl PartialEq for FramePermission {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FramePermission {}

#[cfg(test)]
mod tests;
