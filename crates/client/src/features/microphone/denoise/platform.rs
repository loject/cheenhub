//! Выбор Windows DSP либо нейтральной реализации.

#[cfg(all(
    feature = "windows",
    target_os = "windows",
    target_arch = "x86_64",
    target_env = "msvc"
))]
#[path = "windows.rs"]
mod implementation;
#[cfg(not(all(
    feature = "windows",
    target_os = "windows",
    target_arch = "x86_64",
    target_env = "msvc"
)))]
#[path = "unsupported_impl.rs"]
mod implementation;

#[cfg(not(all(
    feature = "windows",
    target_os = "windows",
    target_arch = "x86_64",
    target_env = "msvc"
)))]
use implementation::Processor as SelectedProcessor;
#[cfg(all(
    feature = "windows",
    target_os = "windows",
    target_arch = "x86_64",
    target_env = "msvc"
))]
use implementation::WindowsProcessor as SelectedProcessor;

/// Процессор, экспортируемый в общий контракт без условий платформы.
pub(super) struct Processor(SelectedProcessor);

impl Processor {
    /// Создаёт процессор, изолированный от platform selection вызывающего слоя.
    pub(super) fn new() -> Self {
        Self(SelectedProcessor::new())
    }

    /// Передаёт платформенной реализации один PCM-кадр.
    pub(super) fn process(&mut self, frame: &mut [f32]) -> Result<(), &'static str> {
        self.0.process(frame)
    }

    /// Указывает, доступна ли DSP реализация для выбранного target.
    pub(super) fn available() -> bool {
        cfg!(all(
            feature = "windows",
            target_os = "windows",
            target_arch = "x86_64",
            target_env = "msvc"
        ))
    }
}
