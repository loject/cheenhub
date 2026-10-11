//! DSP RNNoise на базе nnnoiseless для моно PCM 48 кГц.

use nnnoiseless::DenoiseState;

const FRAME_SAMPLES: usize = 480;

/// Состояние nnnoiseless, изолированное на одну encoder-сессию.
pub(super) struct RnnoiseProcessor {
    state: Box<DenoiseState<'static>>,
    has_processed_frame: bool,
}

impl RnnoiseProcessor {
    /// Создаёт внутреннее состояние денойзера без recoverable init-ошибок.
    pub(super) fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            has_processed_frame: false,
        }
    }

    /// Выполняет DSP кадра и возвращает вероятность речи в диапазоне 0–1.
    ///
    /// Неверный размер, нечисловые входы или результаты означают отказ обработки.
    pub(super) fn process(&mut self, frame: &mut [f32]) -> Result<Option<f32>, &'static str> {
        if frame.len() != FRAME_SAMPLES || frame.iter().any(|sample| !sample.is_finite()) {
            return Err("invalid nnnoiseless input frame");
        }
        let mut input = [0.0; FRAME_SAMPLES];
        let mut output = [0.0; FRAME_SAMPLES];
        for (target, sample) in input.iter_mut().zip(frame.iter()) {
            *target = sample * 32_768.0;
        }
        let speech_probability = self.state.process_frame(&mut output, &input);
        if !speech_probability.is_finite() {
            return Err("nnnoiseless produced non-finite speech probability");
        }
        if !self.has_processed_frame {
            self.has_processed_frame = true;
            return Ok(Some(speech_probability));
        }
        if output.iter().any(|sample| !sample.is_finite()) {
            return Err("nnnoiseless produced non-finite output");
        }
        for (target, sample) in frame.iter_mut().zip(output.iter()) {
            *target = (sample / 32_768.0).clamp(-1.0, 1.0);
        }
        Ok(Some(speech_probability))
    }
}

#[cfg(test)]
mod tests;
