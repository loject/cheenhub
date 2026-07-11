//! Выбор platform backend'а микрофона.

use std::rc::Rc;

use super::backend::MicrophoneBackend;

#[cfg(feature = "android")]
mod android;
#[cfg(any(
    feature = "android",
    feature = "windows",
    feature = "linux",
    feature = "macos"
))]
mod cpal_capture;
#[cfg(any(
    feature = "android",
    feature = "windows",
    feature = "linux",
    feature = "macos"
))]
pub(super) mod device_key;

/// Возвращает backend микрофона для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn MicrophoneBackend> {
    #[cfg(feature = "web")]
    {
        Rc::new(super::browser::BrowserMicrophoneBackend)
    }

    #[cfg(feature = "android")]
    {
        Rc::new(android::AndroidMicrophoneBackend)
    }

    #[cfg(any(feature = "windows", feature = "linux", feature = "macos"))]
    {
        Rc::new(cpal_capture::CpalMicrophoneBackend)
    }

    #[cfg(not(any(
        feature = "web",
        feature = "android",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    )))]
    {
        Rc::new(super::unsupported::UnavailableMicrophoneBackend)
    }
}
