//! Алгоритмы подавления шума и сохраняемый выбор пользователя.

mod dsp;
mod mode;
mod runtime;
pub(crate) use runtime::RuntimeSwitch;
mod platform;
pub(super) use dsp::Processor;
pub(crate) use dsp::StreamProcessor;
pub(crate) use mode::DenoiseMode;

/// Указывает, доступен ли RNNoise на текущей платформе.
pub(crate) fn available() -> bool {
    Processor::available()
}

/// Возвращает начальный режим только для отсутствующей настройки.
pub(crate) fn default_mode() -> DenoiseMode {
    platform::default_mode()
}

/// Указывает, доступен ли отдельный режим встроенного browser-шумоподавления.
pub(crate) fn browser_available() -> bool {
    platform::browser_available()
}

/// Нормализует сохранённый выбор относительно доступных алгоритмов этой сборки.
pub(crate) fn supported_mode(mode: DenoiseMode) -> DenoiseMode {
    mode.supported(available(), browser_available())
}
