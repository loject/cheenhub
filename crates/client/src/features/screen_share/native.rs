//! Выбор платформенной реализации backend'а демонстрации экрана.

use std::rc::Rc;

use super::backend::ScreenShareBackend;

/// Возвращает backend демонстрации экрана для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn ScreenShareBackend> {
    default_backend_platform()
}

#[cfg(target_arch = "wasm32")]
fn default_backend_platform() -> Rc<dyn ScreenShareBackend> {
    Rc::new(super::browser::BrowserScreenShareBackend)
}

#[cfg(target_os = "android")]
fn default_backend_platform() -> Rc<dyn ScreenShareBackend> {
    Rc::new(super::android::AndroidScreenShareBackend)
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn default_backend_platform() -> Rc<dyn ScreenShareBackend> {
    Rc::new(super::unsupported::UnavailableScreenShareBackend)
}
