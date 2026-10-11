//! Выбор RNNoise для Windows, Linux и веба либо нейтральной реализации.

#[cfg(any(
    feature = "web",
    feature = "microphone_worker",
    all(feature = "linux", target_os = "linux"),
    all(
        feature = "windows",
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )
))]
#[path = "rnnoise/mod.rs"]
mod implementation;
#[cfg(not(any(
    feature = "web",
    feature = "microphone_worker",
    all(feature = "linux", target_os = "linux"),
    all(
        feature = "windows",
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )
)))]
#[path = "unsupported_impl.rs"]
mod implementation;

#[cfg(not(any(
    feature = "web",
    feature = "microphone_worker",
    all(feature = "linux", target_os = "linux"),
    all(
        feature = "windows",
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )
)))]
use implementation::Processor as SelectedProcessor;
#[cfg(any(
    feature = "web",
    feature = "microphone_worker",
    all(feature = "linux", target_os = "linux"),
    all(
        feature = "windows",
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )
))]
use implementation::RnnoiseProcessor as SelectedProcessor;

/// Процессор, экспортируемый в общий контракт без условий платформы.
pub(crate) struct Processor(SelectedProcessor);

impl Processor {
    /// Создаёт процессор, изолированный от platform selection вызывающего слоя.
    pub(crate) fn new() -> Self {
        Self(SelectedProcessor::new())
    }

    /// Обрабатывает PCM-кадр и возвращает доступную оценку вероятности речи.
    pub(crate) fn process(&mut self, frame: &mut [f32]) -> Result<Option<f32>, &'static str> {
        self.0.process(frame)
    }

    /// Указывает, доступна ли DSP реализация для выбранного target.
    pub(crate) fn available() -> bool {
        cfg!(any(
            feature = "web",
            feature = "microphone_worker",
            all(feature = "linux", target_os = "linux"),
            all(
                feature = "windows",
                target_os = "windows",
                target_arch = "x86_64",
                target_env = "msvc"
            )
        ))
    }
}
