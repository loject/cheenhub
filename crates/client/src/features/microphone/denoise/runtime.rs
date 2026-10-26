//! Атомарное состояние DSP: поколение переключения защищает от запоздалых отказов.

use std::sync::atomic::{AtomicU64, Ordering};

/// Состояние RNNoise одной capture-сессии, читаемое на границах PCM-кадров.
///
/// Младшие биты содержат enabled/failed; старшие — поколение переключения.
#[derive(Debug)]
pub(crate) struct RuntimeSwitch(AtomicU64);

impl RuntimeSwitch {
    /// Создаёт переключатель без зарегистрированного отказа.
    pub(crate) fn new(enabled: bool) -> Self {
        Self(AtomicU64::new(u64::from(enabled)))
    }

    /// Возвращает текущее разрешение обработки.
    pub(crate) fn load(&self, _ordering: Ordering) -> bool {
        self.snapshot() & 1 != 0
    }

    /// Применяет новый режим и разрешает повторное включение после отказа.
    pub(crate) fn store(&self, enabled: bool, _ordering: Ordering) {
        let _ = self
            .0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                Some(((old >> 2).wrapping_add(1) << 2) | u64::from(enabled))
            });
    }

    /// Возвращает снимок для привязки результата DSP к поколению переключения.
    pub(crate) fn snapshot(&self) -> u64 {
        self.0.load(Ordering::Acquire)
    }

    /// Отмечает отказ только актуального включённого DSP.
    pub(crate) fn fail(&self, snapshot: u64) -> bool {
        snapshot & 1 != 0
            && self
                .0
                .compare_exchange(
                    snapshot,
                    (snapshot & !3) | 2,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
    }

    /// Восстанавливает enabled/failed после неуспешного применения, создавая новое поколение.
    pub(crate) fn restore(&self, snapshot: u64) {
        let _ = self
            .0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |old| {
                Some(((old >> 2).wrapping_add(1) << 2) | (snapshot & 3))
            });
    }

    /// Указывает, отключился ли DSP из-за отказа, а не выбора пользователя.
    pub(crate) fn failed(&self) -> bool {
        self.snapshot() & 2 != 0
    }
}

#[cfg(test)]
mod tests;
