//! DSP микрофона для native runtime и WASM worker, без UI и хранения настроек.

mod platform;
pub(crate) use platform::Processor;

/// Обрабатывает поток кадров PCM 48 кГц с живым переключением RNNoise.
///
/// Каждый chunk должен содержать целое число кадров по 480 samples.
/// Ошибка сохраняет исходный PCM; следующее включение создаёт чистое состояние DSP.
pub(crate) struct StreamProcessor {
    processor: Processor,
    enabled: bool,
    scratch: Vec<f32>,
}

impl StreamProcessor {
    /// Создаёт отключённый обработчик для новой capture-сессии.
    pub(crate) fn new() -> Self {
        Self {
            processor: Processor::new(),
            enabled: false,
            scratch: Vec::new(),
        }
    }

    /// Возвращает среднюю вероятность речи кадров либо `None` при выключенном DSP.
    ///
    /// При ошибке размера или DSP исходный PCM остаётся неизменным.
    pub(crate) fn process(
        &mut self,
        samples: &mut [f32],
        enabled: bool,
    ) -> Result<Option<f32>, &'static str> {
        if !enabled {
            self.enabled = false;
            return Ok(None);
        }
        if !self.enabled {
            self.processor = Processor::new();
        }
        self.enabled = true;
        let result = self.process_chunk(samples);
        if result.is_err() {
            self.enabled = false;
        }
        result
    }

    fn process_chunk(&mut self, samples: &mut [f32]) -> Result<Option<f32>, &'static str> {
        if !Processor::available() {
            return Ok(None);
        }
        if samples.is_empty() || !samples.len().is_multiple_of(480) {
            return Err("RNNoise requires complete 10 ms frames at 48 kHz");
        }
        self.scratch.resize(samples.len(), 0.0);
        self.scratch.copy_from_slice(samples);
        let mut total = 0.0;
        let mut count = 0_u32;
        for frame in self.scratch.as_chunks_mut::<480>().0 {
            if let Some(probability) = self.processor.process(frame)? {
                total += probability;
                count += 1;
            }
        }
        samples.copy_from_slice(&self.scratch);
        Ok((count > 0).then(|| total / count as f32))
    }
}

#[cfg(test)]
mod tests;
