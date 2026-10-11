//! Согласованное состояние portal gate и поколений PCM, не зависящее от D-Bus.

use crate::features::microphone::push_to_talk::gate::Gate;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

/// Синхронизирует события портала с захватом и кодированием аудио.
#[derive(Default)]
pub(super) struct State {
    inner: Mutex<Inner>,
}
#[derive(Default)]
struct Inner {
    gate: Gate,
    epoch: u64,
    heartbeat: Option<Instant>,
}
impl State {
    /// Восстанавливает источник ввода с закрытым gate до нового нажатия.
    pub(super) fn recover(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.epoch = inner.epoch.wrapping_add(1);
        inner.gate.recover(false);
        inner.heartbeat = Some(Instant::now());
    }
    /// Принимает состояние удержания подтверждённого shortcut.
    pub(super) fn event(&self, held: bool) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let previous = inner.gate.active();
        inner.gate.event(held);
        if previous != inner.gate.active() {
            inner.epoch = inner.epoch.wrapping_add(1);
        }
    }
    /// Отзывает все ранее захваченные фрагменты и закрывает gate.
    pub(super) fn fail(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.gate.failed();
        inner.epoch = inner.epoch.wrapping_add(1);
        inner.heartbeat = None;
    }
    /// Подтверждает исправность D-Bus event loop.
    pub(super) fn heartbeat(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if inner
            .heartbeat
            .is_some_and(|beat| beat.elapsed() >= Duration::from_secs(1))
        {
            inner.gate.recover(false);
            inner.epoch = inner.epoch.wrapping_add(1);
        }
        inner.heartbeat = Some(Instant::now());
    }
    /// Возвращает согласованное поколение при исправном источнике и отсутствии назначения.
    pub(super) fn held_epoch(&self, recording: &AtomicU64) -> Option<(u64, u64)> {
        let inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        let generation = recording.load(Ordering::Acquire);
        (generation.is_multiple_of(2)
            && inner.gate.active()
            && inner
                .heartbeat
                .is_some_and(|beat| beat.elapsed() < Duration::from_secs(1))
            && generation == recording.load(Ordering::Acquire))
        .then_some((inner.epoch, generation))
    }
}
#[cfg(test)]
mod tests;
