//! Windows DSP на базе nnnoiseless для моно PCM 48 кГц.

use nnnoiseless::DenoiseState;

const FRAME_SAMPLES: usize = 480;

/// Состояние nnnoiseless, изолированное на одну encoder-сессию.
pub(super) struct WindowsProcessor {
    state: Box<DenoiseState<'static>>,
    has_processed_frame: bool,
}

impl WindowsProcessor {
    /// Создаёт внутреннее состояние денойзера без recoverable init-ошибок.
    pub(super) fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            has_processed_frame: false,
        }
    }

    /// Выполняет DSP одного кадра; несоответствие конфигурации/выхода — отказ обработки.
    pub(super) fn process(&mut self, frame: &mut [f32]) -> Result<(), &'static str> {
        if frame.len() != FRAME_SAMPLES || frame.iter().any(|sample| !sample.is_finite()) {
            return Err("invalid nnnoiseless input frame");
        }
        let mut input = [0.0; FRAME_SAMPLES];
        let mut output = [0.0; FRAME_SAMPLES];
        for (target, sample) in input.iter_mut().zip(frame.iter()) {
            *target = sample * 32_768.0;
        }
        self.state.process_frame(&mut output, &input);
        if !self.has_processed_frame {
            self.has_processed_frame = true;
            return Ok(());
        }
        if output.iter().any(|sample| !sample.is_finite()) {
            return Err("nnnoiseless produced non-finite output");
        }
        for (target, sample) in frame.iter_mut().zip(output.iter()) {
            *target = (sample / 32_768.0).clamp(-1.0, 1.0);
        }
        Ok(())
    }
}
