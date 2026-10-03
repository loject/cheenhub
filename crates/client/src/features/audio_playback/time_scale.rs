//! Сжатие времени входящего голоса без сдвига тона.
//!
//! Модуль содержит потоковый компрессор времени на основе overlap-add с поиском
//! наилучшего совмещения перекрытий (WSOLA) и нелинейный расчёт коэффициента
//! сжатия по объёму избытка буфера воспроизведения. Компрессор сохраняет высоту
//! тона, поэтому ускорение не слышно на слух. Сжатие включается только когда буфер
//! воспроизведения отстаёт от сети, и при коэффициенте ровно `1.0` сигнал проходит
//! через модуль без изменений.

use std::f64::consts::PI;

/// Максимальный коэффициент сжатия времени: примерно +3.1 полутона.
const MAX_TIME_COMPRESSION: f64 = 1.2;
/// Минимальный коэффициент сжатия: едва слышное ускорение при малом отставании.
const GENTLE_TIME_COMPRESSION: f64 = 1.02;
/// Отставание, меньше которого сжатие не включается совсем, в секундах.
const ENGAGE_DEADZONE_SECONDS: f64 = 0.002;
/// Отставание, до которого держится мягкий коэффициент, в секундах.
const GENTLE_EXCESS_SECONDS: f64 = 0.02;
/// Отставание, на котором достигается максимальный коэффициент, в секундах.
const FULL_EXCESS_SECONDS: f64 = 0.2;
/// Порог, ниже которого коэффициент сжатия считается равным единице.
const COMPRESSION_EPSILON: f64 = 1.0e-4;
/// Длина аналитического окна в миллисекундах.
const WINDOW_MILLISECONDS: f64 = 10.0;
/// Радиус поиска совмещения как доля окна: перекрывает период основного тона голоса.
const SEARCH_RADIUS_DIVISOR: usize = 1;
/// Шаг перебора кандидатов как доля окна: ~40 мкс при 48 кГц.
const SEARCH_STEP_DIVISOR: usize = 240;
/// Длина участка корреляции как доля окна: ~1 мс при 48 кГц.
const CORRELATION_DIVISOR: usize = 10;
/// Минимальная частота дискретизации, для которой строится окно.
const MIN_SAMPLE_RATE_HZ: u32 = 8_000;
/// Предел входного буфера в окнах: защищает от роста памяти, если сжатие не успевает.
const MAX_INPUT_WINDOWS: usize = 16;

/// Возвращает коэффициент сжатия времени для избытка буфера в секундах.
///
/// Закон намеренно нелинейный: при малом отставании голос ускоряется едва заметно,
/// при большом отставании коэффициент быстро доходит до максимального, чтобы
/// накопленная задержка не росла. Возвращает `1.0`, когда отставание укладывается в
/// мёртвую зону или неизмеримо.
pub(super) fn catch_up_rate(excess_seconds: f64) -> f64 {
    if !excess_seconds.is_finite() || excess_seconds <= ENGAGE_DEADZONE_SECONDS {
        return 1.0;
    }
    if excess_seconds <= GENTLE_EXCESS_SECONDS {
        return GENTLE_TIME_COMPRESSION;
    }
    if excess_seconds >= FULL_EXCESS_SECONDS {
        return MAX_TIME_COMPRESSION;
    }

    let progress =
        (excess_seconds - GENTLE_EXCESS_SECONDS) / (FULL_EXCESS_SECONDS - GENTLE_EXCESS_SECONDS);
    GENTLE_TIME_COMPRESSION + (MAX_TIME_COMPRESSION - GENTLE_TIME_COMPRESSION) * progress * progress
}

/// Потоковый компрессор времени одного отправителя с сохранением тона.
///
/// Компрессор накапливает входной сигнал, синтезирует перекрывающиеся окна с шагом
/// синтеза `hop` и шагом анализа `hop * rate`, поэтому тембр не меняется, а
/// длительность сигнала сокращается в `rate` раз. Синтез идёт с задержкой в одно
/// окно, поэтому при выключенном сжатии (`rate == 1.0`) сигнал проходит без
/// изменений и без лишней задержки.
pub(super) struct VoiceTimeCompressor {
    /// Окно Ханна синтеза: сумма двух соседних половин равна единице.
    window: Vec<f32>,
    /// Длина окна в сэмплах.
    window_len: usize,
    /// Шаг синтеза и перекрытие в сэмплах.
    hop: usize,
    /// Радиус поиска наилучшего совмещения в сэмплах.
    search_radius: usize,
    /// Шаг перебора кандидатов совмещения в сэмплах.
    search_step: usize,
    /// Длина участка, по которому оценивается совпадение кандидатов, в сэмплах.
    correlation_len: usize,
    /// Входной буфер вместе с ещё не израсходованной историей.
    input: Vec<f32>,
    /// Номинальная позиция чтения следующего окна во входном буфере.
    next_read: f64,
    /// Позиция, из которой было прочитано предыдущее окно.
    previous_read: Option<f64>,
    /// Накопленный overlap-add выход, ещё не переданный вызывающему коду.
    output: Vec<f32>,
    /// Абсолютная позиция выхода, соответствующая `output[0]`.
    emitted: usize,
    /// Количество синтезированных окон.
    windows: usize,
    /// Хвост уже выведенного сигнала для бесшовного старта сжатия.
    tail: Vec<f32>,
    /// Запрошенный коэффициент сжатия времени.
    rate: f64,
    /// Признак того, что модуль уже прогревался входным сигналом.
    primed: bool,
    /// Уже проигранные сэмплы, использованные при прогреве текущей реплики.
    priming_samples: usize,
}

impl VoiceTimeCompressor {
    /// Создаёт компрессор для указанной частоты дискретизации.
    pub(super) fn new(sample_rate_hz: u32) -> Self {
        let sample_rate_hz = sample_rate_hz.max(MIN_SAMPLE_RATE_HZ);
        let window_len =
            ((f64::from(sample_rate_hz) * WINDOW_MILLISECONDS / 1_000.0).round() as usize).max(8);
        let window_len = window_len + window_len % 2;
        let hop = window_len / 2;
        let window = (0..window_len)
            .map(|index| {
                let phase = 2.0 * PI * index as f64 / window_len as f64;
                (0.5 * (1.0 - phase.cos())) as f32
            })
            .collect();

        Self {
            window,
            window_len,
            hop,
            search_radius: (window_len / SEARCH_RADIUS_DIVISOR).max(1),
            search_step: (window_len / SEARCH_STEP_DIVISOR).max(1),
            correlation_len: (window_len / CORRELATION_DIVISOR).max(1),
            input: Vec::new(),
            next_read: 0.0,
            previous_read: None,
            output: Vec::new(),
            emitted: 0,
            windows: 0,
            tail: Vec::new(),
            rate: 1.0,
            primed: false,
            priming_samples: 0,
        }
    }

    /// Возвращает текущий коэффициент сжатия времени.
    pub(super) fn rate(&self) -> f64 {
        self.rate
    }

    /// Возвращает объём уже принятого, но ещё не выведенного контента в сэмплах.
    ///
    /// Нужен регулятору, чтобы измерять отставание в единицах исходного контента:
    /// контент внутри компрессора пользователь слышит с той же задержкой, что и
    /// контент, уже запланированный на воспроизведение.
    pub(super) fn pending_content_samples(&self) -> usize {
        let played_content = self
            .previous_read
            .map_or(self.priming_samples as f64, |read| read + self.hop as f64);
        ((self.input.len() as f64 - played_content).max(0.0) as usize).min(self.input.len())
    }

    /// Задаёт коэффициент сжатия времени, ограничивая его допустимым диапазоном.
    pub(super) fn set_rate(&mut self, rate: f64) {
        self.rate = if rate.is_finite() {
            rate.clamp(1.0, MAX_TIME_COMPRESSION)
        } else {
            1.0
        };
    }

    /// Сбрасывает накопленное состояние, сохраняя коэффициент сжатия.
    pub(super) fn reset(&mut self) {
        self.input.clear();
        self.output.clear();
        self.next_read = 0.0;
        self.previous_read = None;
        self.emitted = 0;
        self.windows = 0;
        self.primed = false;
        self.priming_samples = 0;
    }

    /// Доигрывает оставшийся PCM после паузы и полностью завершает текущую реплику.
    pub(super) fn flush(&mut self) -> Vec<f32> {
        let remaining_from = self
            .previous_read
            .map_or(self.priming_samples, |read| {
                (read + self.hop as f64).round().max(0.0) as usize
            })
            .min(self.input.len());
        let output = self.input[remaining_from..].to_vec();
        self.reset();
        self.tail.clear();
        self.rate = 1.0;
        output
    }

    /// Сжимает один декодированный фрейм и возвращает PCM для воспроизведения.
    ///
    /// При коэффициенте `1.0` вход возвращается без изменений и без задержки.
    /// При переходе обратно в `1.0` накопленный остаток доигрывается и состояние
    /// сбрасывается, поэтому стык сжатого и несжатого сигнала остаётся непрерывным.
    pub(super) fn process(&mut self, samples: &[f32]) -> Vec<f32> {
        let rate = self.rate;
        let compressing = rate > 1.0 + COMPRESSION_EPSILON;
        if !self.primed {
            if !compressing {
                self.remember_tail(samples);
                return samples.to_vec();
            }
            self.prime();
        }

        if self.input.len().saturating_add(samples.len()) > self.window_len * MAX_INPUT_WINDOWS {
            self.reset();
            self.prime();
        }
        self.input.extend_from_slice(samples);
        self.synthesize(rate);
        let mut output = self.take_output();
        if !compressing && !self.can_form_window() {
            let remaining_from = (self.next_read.round().max(0.0) as usize).min(self.input.len());
            output.extend_from_slice(&self.input[remaining_from..]);
            self.reset();
            self.remember_tail(&output);
            return output;
        }

        self.prune_input();
        output
    }

    /// Прогревает компрессор хвостом уже выведенного сигнала.
    fn prime(&mut self) {
        self.primed = true;
        self.priming_samples = self.tail.len();
        self.input.clear();
        self.input.extend_from_slice(&self.tail);
    }

    /// Синтезирует все окна, для которых во входном буфере хватает данных.
    fn synthesize(&mut self, rate: f64) {
        let analysis_hop = self.hop as f64 * rate;
        let search = rate > 1.0 + COMPRESSION_EPSILON;
        while self.can_form_window() {
            let read = self.choose_read(self.next_read, search);
            let out_pos = self.windows * self.hop;
            self.write_window(read, out_pos);
            self.previous_read = Some(read as f64);
            self.next_read += analysis_hop;
            self.windows += 1;
        }
    }

    /// Выбирает позицию чтения окна, совмещая перекрытие с предыдущим окном.
    fn choose_read(&self, nominal: f64, search: bool) -> usize {
        let max_start = self.input.len().saturating_sub(self.window_len);
        let nominal = nominal.round().clamp(0.0, max_start as f64) as usize;
        if !search {
            return nominal;
        }
        let Some(previous_read) = self.previous_read else {
            return nominal;
        };

        let correlation_len = self.correlation_len.min(max_start.saturating_add(1));
        let target = (previous_read + self.hop as f64).clamp(0.0, max_start as f64) as usize;
        let low = nominal.saturating_sub(self.search_radius);
        let high = nominal
            .saturating_add(self.search_radius)
            .min(max_start.saturating_sub(correlation_len.saturating_sub(1)));
        let mut best_read = nominal;
        let mut best_similarity = f32::NEG_INFINITY;
        let mut candidate = low;
        while candidate <= high {
            let similarity = self.similarity(candidate, target, correlation_len);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_read = candidate;
            }
            candidate += self.search_step;
        }
        best_read
    }

    /// Возвращает нормированную корреляцию двух участков входа.
    fn similarity(&self, left: usize, right: usize, length: usize) -> f32 {
        let mut cross = 0.0_f32;
        let mut left_energy = 0.0_f32;
        let mut right_energy = 0.0_f32;
        for offset in 0..length {
            let left_sample = self.input[left + offset];
            let right_sample = self.input[right + offset];
            cross += left_sample * right_sample;
            left_energy += left_sample * left_sample;
            right_energy += right_sample * right_sample;
        }
        if left_energy <= f32::EPSILON || right_energy <= f32::EPSILON {
            return 0.0;
        }

        cross / (left_energy * right_energy).sqrt()
    }

    /// Складывает окно в накопленный выход с коэффициентом Ханна.
    fn write_window(&mut self, read: usize, out_pos: usize) {
        let out_offset = out_pos - self.emitted;
        let required = out_offset + self.window_len;
        if self.output.len() < required {
            self.output.resize(required, 0.0);
        }
        for index in 0..self.window_len {
            self.output[out_offset + index] += self.input[read + index] * self.window[index];
        }
    }

    /// Забирает из накопленного выхода все образцы с полным перекрытием.
    fn take_output(&mut self) -> Vec<f32> {
        if self.emitted < self.hop {
            let warmup = (self.hop - self.emitted).min(self.output.len());
            self.output.drain(..warmup);
            self.emitted += warmup;
        }

        let take = (self.windows * self.hop)
            .saturating_sub(self.emitted)
            .min(self.output.len());
        let output = self.output.drain(..take).collect::<Vec<f32>>();
        self.emitted += take;
        self.remember_tail(&output);
        output
    }

    /// Освобождает входной буфер от уже израсходованной истории.
    fn prune_input(&mut self) {
        let history = (self.window_len + self.search_radius) as f64;
        let prune_from = (self.next_read - history).floor().max(0.0) as usize;
        if prune_from == 0 {
            return;
        }

        self.input.drain(..prune_from);
        self.next_read -= prune_from as f64;
        if let Some(previous_read) = self.previous_read.as_mut() {
            *previous_read -= prune_from as f64;
        }
    }

    /// Запоминает хвост выведенного сигнала для последующего старта сжатия.
    fn remember_tail(&mut self, played: &[f32]) {
        let keep = self.hop.min(played.len());
        self.tail.clear();
        self.tail
            .extend_from_slice(&played[played.len().saturating_sub(keep)..played.len()]);
    }

    /// Хватает ли входного буфера, чтобы синтезировать ещё одно окно.
    fn can_form_window(&self) -> bool {
        self.next_read + self.window_len as f64 <= self.input.len() as f64
    }
}

#[cfg(test)]
mod tests;
