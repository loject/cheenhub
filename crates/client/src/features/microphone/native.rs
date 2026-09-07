//! Выбор platform backend'а микрофона.

use std::rc::Rc;

use super::backend::MicrophoneBackend;

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "android", feature = "windows", feature = "macos"))]
mod cpal_capture;
#[cfg(any(target_os = "android", feature = "windows", feature = "macos"))]
pub(super) mod device_key;
#[cfg(any(
    target_os = "android",
    feature = "windows",
    feature = "linux",
    feature = "macos"
))]
mod encoding;
#[cfg(feature = "linux")]
mod linux;

/// Возвращает backend микрофона для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn MicrophoneBackend> {
    #[cfg(feature = "web")]
    {
        Rc::new(super::browser::BrowserMicrophoneBackend)
    }

    #[cfg(target_os = "android")]
    {
        Rc::new(android::AndroidMicrophoneBackend)
    }

    #[cfg(feature = "linux")]
    {
        Rc::new(linux::PulseMicrophoneBackend)
    }

    #[cfg(any(feature = "windows", feature = "macos"))]
    {
        Rc::new(cpal_capture::CpalMicrophoneBackend)
    }

    #[cfg(not(any(
        feature = "web",
        target_os = "android",
        feature = "windows",
        feature = "linux",
        feature = "macos"
    )))]
    {
        Rc::new(super::unsupported::UnavailableMicrophoneBackend)
    }
}
