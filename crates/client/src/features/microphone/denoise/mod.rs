//! Контракт подавления шума и платформенные реализации.

mod mode;
mod platform;
pub(crate) use mode::DenoiseMode;

use platform::Processor as PlatformProcessor;

/// Обработчик кадров микрофона, принадлежащий encoder worker-сессии.
pub(super) struct Processor(Implementation);

enum Implementation {
    Platform(PlatformProcessor),
}

impl Processor {
    /// Указывает, доступен ли denoiser на текущей выбранной сборке.
    pub(super) fn available() -> bool {
        PlatformProcessor::available()
    }

    /// Создаёт чистое состояние DSP для новой encoder-сессии.
    pub(super) fn new() -> Self {
        Self(implementation())
    }

    /// Обрабатывает один 10 мс кадр в нормализованной float PCM.
    pub(super) fn process(&mut self, frame: &mut [f32]) -> Result<(), &'static str> {
        match &mut self.0 {
            Implementation::Platform(processor) => processor.process(frame),
        }
    }
}

pub(crate) fn available() -> bool {
    Processor::available()
}

fn implementation() -> Implementation {
    Implementation::Platform(PlatformProcessor::new())
}

#[cfg(test)]
mod tests;
