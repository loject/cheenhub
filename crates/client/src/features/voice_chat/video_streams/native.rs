//! Выбор платформенной реализации renderer'а видео участников.

use std::rc::Rc;

use super::backend::ParticipantVideoBackend;

/// Возвращает backend renderer'а видео участников для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn ParticipantVideoBackend> {
    default_backend_platform()
}

#[cfg(target_arch = "wasm32")]
fn default_backend_platform() -> Rc<dyn ParticipantVideoBackend> {
    Rc::new(super::web::WebParticipantVideoBackend)
}

#[cfg(not(target_arch = "wasm32"))]
fn default_backend_platform() -> Rc<dyn ParticipantVideoBackend> {
    Rc::new(super::unsupported::UnavailableParticipantVideoBackend)
}
