//! Выбор платформенной реализации backend'а микрофона.

use std::rc::Rc;

use super::backend::MicrophoneBackend;

/// Возвращает backend микрофона для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn MicrophoneBackend> {
    default_backend_platform()
}

#[cfg(target_arch = "wasm32")]
fn default_backend_platform() -> Rc<dyn MicrophoneBackend> {
    Rc::new(super::browser::BrowserMicrophoneBackend)
}

#[cfg(not(target_arch = "wasm32"))]
fn default_backend_platform() -> Rc<dyn MicrophoneBackend> {
    Rc::new(super::unsupported::UnavailableMicrophoneBackend)
}
