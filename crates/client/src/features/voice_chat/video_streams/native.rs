//! Выбор платформенной реализации renderer'а видео участников.

use std::rc::Rc;

use super::backend::ParticipantVideoBackend;

#[cfg(all(target_os = "windows", feature = "windows"))]
mod windows;

#[cfg(all(
    not(target_arch = "wasm32"),
    not(all(target_os = "windows", feature = "windows"))
))]
mod unsupported;

/// Возвращает backend renderer'а видео участников для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn ParticipantVideoBackend> {
    default_backend_platform()
}

#[cfg(target_arch = "wasm32")]
fn default_backend_platform() -> Rc<dyn ParticipantVideoBackend> {
    Rc::new(super::web::WebParticipantVideoBackend)
}

#[cfg(all(target_os = "windows", feature = "windows"))]
fn default_backend_platform() -> Rc<dyn ParticipantVideoBackend> {
    Rc::new(windows::WindowsParticipantVideoBackend)
}

#[cfg(all(
    not(target_arch = "wasm32"),
    not(all(target_os = "windows", feature = "windows"))
))]
fn default_backend_platform() -> Rc<dyn ParticipantVideoBackend> {
    Rc::new(unsupported::UnavailableParticipantVideoBackend)
}
