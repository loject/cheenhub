//! Неподдерживаемый backend камеры.

use std::rc::Rc;

use futures_util::FutureExt;
use futures_util::future::LocalBoxFuture;

use super::backend::{CameraBackend, CameraCallbacks, CameraConfig, CameraError, CameraSession};

/// Backend камеры для платформ без поддержки захвата.
pub(crate) struct UnavailableCameraBackend;

impl CameraBackend for UnavailableCameraBackend {
    fn start(
        &self,
        _config: CameraConfig,
        callbacks: CameraCallbacks,
    ) -> LocalBoxFuture<'static, Result<Rc<dyn CameraSession>, CameraError>> {
        let CameraCallbacks {
            on_frame: _on_frame,
            on_ended: _on_ended,
        } = callbacks;

        async move { Err(CameraError::new("Камера недоступна.")) }.boxed_local()
    }
}
