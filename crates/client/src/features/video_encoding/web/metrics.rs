//! Ограничение очереди и измерение задержки браузерного кодировщика.

use js_sys::{Function, Reflect};
use std::collections::VecDeque;
use wasm_bindgen::{JsCast, JsValue};

// Два ожидающих вызова дают кодировщику небольшой запас, не накапливая
// длинную очередь; кадр, обрабатываемый самим браузером, сюда не входит.
const MAX_QUEUED_FRAMES: u32 = 2;
const MAX_TIMINGS: usize = 64;

#[derive(Default)]
/// Состояние backpressure и ограниченная история времени кодирования.
pub(super) struct EncoderLoad {
    /// Число кадров, пропущенных из-за очереди.
    pub(super) dropped: u64,
    pending_key: bool,
    timings: VecDeque<(u64, f64)>,
}

impl EncoderLoad {
    /// Удаляет незавершённые замеры прежней конфигурации.
    pub(super) fn clear_timings(&mut self) {
        self.timings.clear();
    }
    /// Проверяет очередь и сохраняет запрос keyframe при пропуске кадра.
    pub(super) fn accept(&mut self, queue: u32, key: bool) -> bool {
        if queue < MAX_QUEUED_FRAMES {
            return true;
        }
        self.dropped = self.dropped.saturating_add(1);
        self.pending_key |= key;
        false
    }

    /// Объединяет текущий и отложенный запросы keyframe.
    pub(super) fn take_key_frame(&mut self, key: bool) -> bool {
        std::mem::take(&mut self.pending_key) || key
    }

    /// Требует keyframe для следующего принятого кадра.
    pub(super) fn require_key_frame(&mut self) {
        self.pending_key = true;
    }

    /// Запоминает монотонное время отправки кадра в миллисекундах.
    pub(super) fn submitted(&mut self, timestamp: u64, now: f64) {
        if self.timings.len() == MAX_TIMINGS {
            self.timings.pop_front();
        }
        self.timings.push_back((timestamp, now));
    }

    /// Возвращает задержку кодирования в миллисекундах для известной метки кадра.
    pub(super) fn output_latency(&mut self, timestamp: u64, now: f64) -> Option<f64> {
        let index = self
            .timings
            .iter()
            .position(|(candidate, _)| *candidate == timestamp)?;
        self.timings
            .remove(index)
            .map(|(_, start)| (now - start).max(0.0))
    }
}

/// Оценивает возраст кадра в миллисекундах на шкале performance API.
pub(super) fn frame_age_ms(timestamp_us: u64) -> f64 {
    performance_now_ms()
        .map(|now_ms| (now_ms - timestamp_us as f64 / 1_000.0).max(0.0))
        .unwrap_or(-1.0)
}

/// Возвращает монотонное время браузера в миллисекундах, если оно доступно.
pub(super) fn performance_now_ms() -> Option<f64> {
    let performance = Reflect::get(&js_sys::global(), &JsValue::from_str("performance")).ok()?;
    Reflect::get(&performance, &JsValue::from_str("now"))
        .ok()?
        .dyn_into::<Function>()
        .ok()?
        .call0(&performance)
        .ok()?
        .as_f64()
}

#[cfg(test)]
mod tests;
