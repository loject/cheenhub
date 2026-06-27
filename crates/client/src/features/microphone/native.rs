//! Выбор platform backend'а микрофона.

use std::rc::Rc;

use super::backend::MicrophoneBackend;

#[cfg(any(feature = "windows", feature = "linux", feature = "macos"))]
mod cpal_capture;

/// Возвращает backend микрофона для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn MicrophoneBackend> {
    #[cfg(feature = "web")]
    {
        Rc::new(super::browser::BrowserMicrophoneBackend)
    }

    #[cfg(any(feature = "windows", feature = "linux", feature = "macos"))]
    {
        Rc::new(cpal_capture::CpalMicrophoneBackend)
    }

    #[cfg(not(any(
        feature = "web",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    )))]
    {
        Rc::new(super::unsupported::UnavailableMicrophoneBackend)
    }
}
