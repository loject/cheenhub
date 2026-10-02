//! Цель PCM-планировщика и отменяемый срок доигрывания хвоста после паузы.

use std::cell::Cell;
use std::rc::Rc;

use super::backend::target_playout_depth_seconds;

/// Минимальный стартовый запас PCM-планировщика в секундах.
pub(super) const STARTUP_PLAYOUT_MARGIN_SECONDS: f64 = 0.02;

/// Учитывает минимальный запас PCM-планировщика в цели сжатия времени.
pub(super) fn scheduled_target_depth_seconds(jitter_buffer_us: u32) -> f64 {
    target_playout_depth_seconds(jitter_buffer_us).max(STARTUP_PLAYOUT_MARGIN_SECONDS)
}

/// Выбирает старт PCM в шкале AudioContext, в секундах.
///
/// Пока предыдущий буфер ещё играет, следующий начинается сразу после него.
/// При старте или underrun добавляется запас от 20 до 30 мс с учётом jitter buffer.
pub(super) fn scheduled_start_seconds(
    now: f64,
    previous_until: Option<f64>,
    jitter_buffer_us: u32,
) -> f64 {
    match previous_until {
        Some(until) if until > now => until,
        _ => now + scheduled_target_depth_seconds(jitter_buffer_us).min(0.03),
    }
}

/// Общий с задачей срок паузы; очистка владельца отменяет ожидающее доигрывание.
#[derive(Clone, Default)]
pub(super) struct PlaybackPause {
    deadline_us: Rc<Cell<Option<u64>>>,
}

impl PlaybackPause {
    /// Продлевает срок паузы; возвращает, нужно ли запустить задачу ожидания.
    pub(super) fn refresh(&self, now_us: u64, duration_us: u32, jitter_buffer_us: u32) -> bool {
        self.deadline_us
            .replace(Some(now_us.saturating_add(
                u64::from(duration_us) + u64::from(jitter_buffer_us),
            )))
            .is_none()
    }

    /// Продлевает только уже активное ожидание при получении нового пакета.
    pub(super) fn postpone(&self, now_us: u64, duration_us: u32, jitter_buffer_us: u32) {
        if self.deadline_us.get().is_some() {
            self.refresh(now_us, duration_us, jitter_buffer_us);
        }
    }

    /// Возвращает остаток ожидания либо отсутствие отменённого срока.
    pub(super) fn remaining_us(&self, now_us: u64) -> Option<u64> {
        self.deadline_us
            .get()
            .map(|deadline| deadline.saturating_sub(now_us))
    }

    /// Забирает наступивший срок ровно один раз.
    pub(super) fn take_due(&self, now_us: u64) -> bool {
        if self.remaining_us(now_us) != Some(0) {
            return false;
        }
        self.cancel();
        true
    }

    /// Отменяет доигрывание, в том числе в задаче с клоном этого состояния.
    pub(super) fn cancel(&self) {
        self.deadline_us.set(None);
    }
}

#[cfg(test)]
#[path = "playout_timing_tests.rs"]
mod tests;
