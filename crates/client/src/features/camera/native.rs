//! Выбор платформенной реализации backend'а камеры.

use std::rc::Rc;

use super::backend::CameraBackend;

/// Возвращает backend камеры для текущей платформы.
pub(super) fn default_backend() -> Rc<dyn CameraBackend> {
    default_backend_platform()
}

#[cfg(target_arch = "wasm32")]
fn default_backend_platform() -> Rc<dyn CameraBackend> {
    Rc::new(super::web::WebCameraBackend)
}

#[cfg(not(target_arch = "wasm32"))]
fn default_backend_platform() -> Rc<dyn CameraBackend> {
    Rc::new(super::unsupported::UnavailableCameraBackend)
}
